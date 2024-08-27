use futures::prelude::*;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use tokio::time::{ self, sleep };

use crate::{ Sender, MarketPrice };

const CONNECT_TIMEOUT: u64 = 5;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait ExchangeWebSocketConfig {
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
            Duration::from_secs(CONNECT_TIMEOUT),
            connect_async(T::url())
        ).await else {
            continue;
        };

        log::debug!("{} connected", T::exchange_id());

        if conn.send(Message::Text(T::get_subscribe_payload(markets.as_ref()))).await.is_err() {
            continue;
        }

        log::debug!("{} subscribed", T::exchange_id());

        while let Some(Ok(message)) = conn.next().await {
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

            if tx.is_closed() {
                let _ = conn.close(None).await;
                break;
            }
        }
    }
}
