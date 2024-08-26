use futures::prelude::*;
use rust_decimal_macros::dec;
use serde::Deserialize;

use std::time::Duration;

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use rust_decimal::Decimal;
use tokio::time::sleep;

use crate::Sender;

const ENDPOINT_URL: &str = "wss://stream.binance.com:9443/ws";

struct Binance;

impl Binance {
    pub async fn run(tx: Sender<(String, Decimal)>, pairs: impl AsRef<[&str]>) {
        while !tx.is_closed() {
            sleep(Duration::from_millis(100)).await;

            log::debug!("connecting...");

            let Ok((mut stream, _)) = connect_async(ENDPOINT_URL).await else {
                continue;
            };

            log::debug!("connected");

            if
                let Err(_) = stream.send(
                    Message::Text(
                        format!(
                            r#"{{"method": "SUBSCRIBE", "params": [{}], "id": 1 }}"#,
                            pairs
                                .as_ref()
                                .iter()
                                .map(|pair| format!(r#""{pair}@bookTicker""#))
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    )
                ).await
            {
                continue;
            }

            log::debug!("subscribed");

            while let Some(Ok(Message::Text(message))) = stream.next().await {
                log::trace!("{message:?}");

                if let Ok(tick) = serde_json::from_str::<BinanceBookTicker>(&message) {
                    let price = tick.price();

                    if tx.send((tick.symbol, price)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct BinanceBookTicker {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "b")]
    bid: Decimal,
    #[serde(rename = "a")]
    ask: Decimal,
}

impl BinanceBookTicker {
    pub fn price(&self) -> Decimal {
        (self.bid + self.ask) / dec!(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger::Env;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        Binance::run(tx, ["btcusdt"]).await;
        assert_eq!(dec!(1), dec!(1));
    }
}
