use futures::prelude::*;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use tokio::time::{ self, sleep };

use crate::{ Sender, MarketPrice };

const CONNECT_TIMEOUT: u64 = 5;

pub trait ExchangeWebSocketConfig {
    fn exchange_id() -> &'static str;
    fn url() -> String;
    fn get_subscribe_payload<'a>(markets: impl AsRef<[&'a str]>) -> String;
    fn parse_incoming_payload(payload: String) -> Option<MarketPrice>;
}

pub async fn websocket_run<T: ExchangeWebSocketConfig>(
    tx: Sender<MarketPrice>,
    markets: impl AsRef<[&str]>
) {
    while !tx.is_closed() {
        sleep(Duration::from_millis(100)).await;

        log::debug!("{} connecting...", T::exchange_id());

        let Ok(Ok((mut stream, _))) = time::timeout(
            Duration::from_secs(CONNECT_TIMEOUT),
            connect_async(T::url())
        ).await else {
            continue;
        };

        log::debug!("{} connected", T::exchange_id());

        if stream.send(Message::Text(T::get_subscribe_payload(markets.as_ref()))).await.is_err() {
            continue;
        }

        log::debug!("{} subscribed", T::exchange_id());

        while let Some(Ok(message)) = stream.next().await {
            log::trace!("{} {message:?}", T::exchange_id());

            match message {
                Message::Text(payload) => {
                    if let Some(value) = T::parse_incoming_payload(payload) {
                        tx.send_replace(value);
                    }
                }
                Message::Ping(value) => {
                    let _ = stream.send(Message::Pong(value)).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }

            if tx.is_closed() {
                let _ = stream.close(None).await;
                return;
            }
        }
    }
}
