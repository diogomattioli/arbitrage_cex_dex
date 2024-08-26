use futures::prelude::*;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use tokio::time::sleep;

use crate::{ Sender, MarketPrice };

pub trait ExchangeWebSocketConfig {
    fn exchange_id() -> &'static str;
    fn url() -> String;
    fn get_subscribe_payload<'a>(pairs: impl AsRef<[&'a str]>) -> String;
    fn parse_incoming_payload(payload: String) -> Option<MarketPrice>;
}

pub async fn websocket_run<T: ExchangeWebSocketConfig>(
    tx: Sender<MarketPrice>,
    pairs: impl AsRef<[&str]>
) {
    while !tx.is_closed() {
        sleep(Duration::from_millis(100)).await;

        log::debug!("{} connecting...", T::exchange_id());

        let Ok((mut stream, _)) = connect_async(T::url()).await else {
            continue;
        };

        log::debug!("{} connected", T::exchange_id());

        if stream.send(Message::Text(T::get_subscribe_payload(pairs.as_ref()))).await.is_err() {
            continue;
        }

        log::debug!("{} subscribed", T::exchange_id());

        while let Some(Ok(message)) = stream.next().await {
            log::trace!("{} {message:?}", T::exchange_id());

            match message {
                Message::Text(payload) => {
                    if let Some(value) = T::parse_incoming_payload(payload) {
                        if tx.send(value).await.is_err() {
                            break;
                        }
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
        }
    }
}
