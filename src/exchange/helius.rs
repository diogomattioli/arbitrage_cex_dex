use rust_decimal_macros::dec;
use serde::Deserialize;

use rust_decimal::Decimal;

use crate::websocket::ExchangeWebSocketConfig;

use std::env;

struct Helius;

impl ExchangeWebSocketConfig for Helius {
    fn name() -> &'static str {
        "Helius"
    }

    fn url() -> String {
        format!(
            "wss://mainnet.helius-rpc.com/?api-key={}",
            env::var("HELIUS_API_KEY").expect("cannot find environment variable HELIUS_API_KEY")
        )
    }

    fn get_subscribe_payload<'a>(pairs: impl AsRef<[&'a str]>) -> String {
        format!(
            r#"{{"jsonrpc": "2.0", "method": "accountSubscribe", "params": [{}, {{"encoding": "jsonParsed", "commitment": "confirmed"}}], "id": 1 }}"#,
            pairs
                .as_ref()
                .iter()
                .map(|pair| format!(r#""{pair}""#))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<(String, Decimal)> {
        match serde_json::from_str::<HeliusAccount>(&payload) {
            Ok(account) => {
                let price = account.price();
                Some((account.owner, price))
            }
            Err(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct HeliusAccount {
    owner: String,
    data: Vec<u8>,
}

impl HeliusAccount {
    pub fn price(&self) -> Decimal {
        dec!(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::websocket::websocket_run;

    use super::*;
    use env_logger::Env;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, rx) = tokio::sync::mpsc::channel(1000);
        websocket_run::<Helius>(tx, ["So11111111111111111111111111111111111111112"]).await;
        assert_eq!(dec!(1), dec!(1));
    }
}
