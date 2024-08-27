use serde::Deserialize;

use rust_decimal::Decimal;

use crate::{ websocket::ExchangeWebSocketConfig, MarketPrice };

use std::env;

pub struct Helius;

impl ExchangeWebSocketConfig for Helius {
    fn exchange_id() -> &'static str {
        "helius"
    }

    fn url() -> String {
        format!(
            "wss://mainnet.helius-rpc.com/?api-key={}",
            env::var("HELIUS_API_KEY").expect("cannot find environment variable HELIUS_API_KEY")
        )
    }

    fn get_subscribe_payload<'a>(markets: &[&'a str]) -> String {
        format!(
            r#"{{"jsonrpc": "2.0", "method": "accountSubscribe", "params": [{}, {{"encoding": "jsonParsed", "commitment": "confirmed"}}], "id": 1 }}"#,
            markets
                .as_ref()
                .iter()
                .map(|market| format!(r#""{market}""#))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<MarketPrice> {
        match serde_json::from_str::<HeliusAccount>(&payload) {
            Ok(account) => {
                let price = account.price();
                Some(MarketPrice { exchange_id: Self::exchange_id(), market: account.owner, price })
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
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::websocket::run_websocket;

    use super::*;
    use env_logger::Env;

    #[ignore]
    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, _rx) = tokio::sync::watch::channel(MarketPrice::default());
        run_websocket::<Helius>(tx, &["So11111111111111111111111111111111111111112"]).await;
    }

    #[test]
    fn test_get_subscribe_payload() {
        let payload = Helius::get_subscribe_payload(
            &["So11111111111111111111111111111111111111112"]
        );
        assert_eq!(
            payload,
            r#"{"jsonrpc": "2.0", "method": "accountSubscribe", "params": ["So11111111111111111111111111111111111111112", {"encoding": "jsonParsed", "commitment": "confirmed"}], "id": 1 }"#
        );
    }
}
