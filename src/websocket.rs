use futures::prelude::*;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use rust_decimal::Decimal;
use tokio::time::sleep;

use crate::Sender;

pub trait ExchangeWebSocketConfig {
    fn name() -> &'static str;
    fn url() -> String;
    fn get_subscribe_payload<'a>(pairs: impl AsRef<[&'a str]>) -> String;
    fn parse_incoming_payload(payload: String) -> Option<(String, Decimal)>;
}

pub async fn websocket_run<T: ExchangeWebSocketConfig>(
    tx: Sender<(String, Decimal)>,
    pairs: impl AsRef<[&str]>
) {
    while !tx.is_closed() {
        sleep(Duration::from_millis(100)).await;

        log::debug!("{} connecting...", T::name());

        let Ok((mut stream, _)) = connect_async(T::url()).await else {
            continue;
        };

        log::debug!("{} connected", T::name());

        if stream.send(Message::Text(T::get_subscribe_payload(pairs.as_ref()))).await.is_err() {
            continue;
        }

        log::debug!("{} subscribed", T::name());

        while let Some(Ok(message)) = stream.next().await {
            log::trace!("{} {message:?}", T::name());

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
