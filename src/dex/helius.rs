use futures::prelude::*;
use rust_decimal_macros::dec;
use serde::Deserialize;

use std::{ env, time::Duration };

use async_tungstenite::{ tokio::connect_async, tungstenite::Message };
use rust_decimal::Decimal;
use tokio::time::sleep;

use crate::Sender;

const ENDPOINT_URL: &str = "wss://mainnet.helius-rpc.com/?api-key=";

struct Helius;

impl Helius {
    pub async fn run(tx: Sender<(String, Decimal)>, pairs: impl AsRef<[&str]>) {
        while !tx.is_closed() {
            sleep(Duration::from_millis(100)).await;

            log::debug!("connecting...");

            let Ok((mut stream, _)) = connect_async(
                format!(
                    "{ENDPOINT_URL}{}",
                    env
                        ::var("HELIUS_API_KEY")
                        .expect("cannot find environment variable HELIUS_API_KEY")
                )
            ).await else {
                continue;
            };

            log::debug!("connected");

            if
                let Err(_) = stream.send(
                    Message::Text(
                        format!(
                            r#"{{"jsonrpc": "2.0", "method": "accountSubscribe", "params": [{}], "id": 1 }}"#,
                            pairs
                                .as_ref()
                                .iter()
                                .map(|pair| format!(r#""{pair}""#))
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

                if let Ok(account) = serde_json::from_str::<HeliusAccountSubscription>(&message) {
                    let price = account.price();

                    if tx.send((account.owner, price)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct HeliusAccountSubscription {
    owner: String,
    data: Vec<u8>,
}

impl HeliusAccountSubscription {
    pub fn price(&self) -> Decimal {
        dec!(0)
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
        Helius::run(tx, ["SysvarC1ock11111111111111111111111111111111"]).await;
        assert_eq!(dec!(1), dec!(1));
    }
}
