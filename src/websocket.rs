use futures::prelude::*;
use stream::FusedStream;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use tokio::{ select, time::{ self, sleep, sleep_until } };

use crate::{ Sender, MarketPrice };

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait ExchangeWebSocketConfig {
    fn connect_timeout() -> Duration {
        Duration::from_secs(5)
    }
    fn ping_interval() -> Duration {
        Duration::from_secs(30)
    }
    fn exchange_id() -> &'static str;
    fn url() -> String;
    fn get_subscribe_payload<'a>(markets: &[&'a str]) -> String;
    fn parse_incoming_payload(payload: String) -> Option<MarketPrice>;
}

pub async fn run_websocket<T: ExchangeWebSocketConfig>(tx: Sender<MarketPrice>, markets: &[&str]) {
    while !tx.is_closed() {
        // sleeping to avoid max cpu usage in case of retry
        sleep(Duration::from_millis(100)).await;

        log::debug!("{} connecting...", T::exchange_id());

        let Ok(Ok((mut conn, _))) = time::timeout(
            T::connect_timeout(),
            connect_async(T::url())
        ).await else {
            continue;
        };

        log::debug!("{} connected", T::exchange_id());

        let mut ping_deadline = time::Instant::now() + T::ping_interval();
        let _ = conn.send(Message::Ping(vec![])).await;
        log::trace!("{} ping", T::exchange_id());

        if conn.send(Message::Text(T::get_subscribe_payload(markets))).await.is_err() {
            continue;
        }

        log::debug!("{} subscribed", T::exchange_id());

        while !tx.is_closed() && !conn.is_terminated() {
            select! {
                // to gracefully exit in max 500 millis
                _ = sleep(Duration::from_millis(500)) => {}
                _ = sleep_until(ping_deadline) => {
                    ping_deadline = time::Instant::now() + T::ping_interval();
                    let _ = conn.send(Message::Ping(vec![])).await;
                    log::trace!("{} ping", T::exchange_id());
                }

                res = conn.next() => {
                    if let Some(Ok(message)) = res {
                        log::trace!("{} {message:?}", T::exchange_id());

                        match message {
                            Message::Text(payload) => {
                                if let Some(market_price) = T::parse_incoming_payload(payload) {
                                    // always replace to the most up-to-date market price
                                    tx.send_replace(market_price);
                                }
                            }
                            Message::Ping(value) => {
                                let _ = conn.send(Message::Pong(value)).await;
                            }
                            Message::Close(_) => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if !conn.is_terminated() {
            let _ = conn.close(None).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use env_logger::Env;
    use mockall::Sequence;
    use tokio::join;
    use ws_mock::{ matchers::StringExact, ws_mock_server::{ WsMock, WsMockServer } };

    use super::*;

    #[tokio::test]
    async fn test_run_websocket() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let server = WsMockServer::start().await;

        let mut seq = Sequence::new();

        let ctx = MockExchangeWebSocketConfig::connect_timeout_context();
        ctx.expect()
            .times(..)
            .return_const(Duration::from_secs(5));

        let ctx = MockExchangeWebSocketConfig::ping_interval_context();
        ctx.expect()
            .times(..)
            .return_const(Duration::from_secs(30));

        let ctx = MockExchangeWebSocketConfig::exchange_id_context();
        ctx.expect()
            .times(..)
            .return_const("test");

        let ctx = MockExchangeWebSocketConfig::url_context();
        ctx.expect().once().return_const(server.uri().await);

        let ctx = MockExchangeWebSocketConfig::get_subscribe_payload_context();
        ctx.expect().once().in_sequence(&mut seq).return_const("test_subscribe".to_string());

        let ctx = MockExchangeWebSocketConfig::parse_incoming_payload_context();
        ctx.expect().once().in_sequence(&mut seq).return_const(None);

        WsMock::new()
            .matcher(StringExact::new("test_subscribe"))
            .respond_with(Message::Text("test_response".to_string()))
            .expect(1)
            .mount(&server).await;

        let (tx, rx) = tokio::sync::watch::channel(MarketPrice::default());

        join!(run_websocket::<MockExchangeWebSocketConfig>(tx, &["btcusdt"]), async move {
            sleep(Duration::from_secs(1)).await;
            drop(rx);
        });

        server.verify().await;
    }
}
