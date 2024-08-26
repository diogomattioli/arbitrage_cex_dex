use rust_decimal_macros::dec;
use serde::Deserialize;

use rust_decimal::Decimal;

use crate::{ websocket::ExchangeWebSocketConfig, MarketPrice };

struct Binance;

impl ExchangeWebSocketConfig for Binance {
    fn exchange_id() -> &'static str {
        "binance"
    }

    fn url() -> String {
        "wss://stream.binance.com:9443/ws".to_string()
    }

    fn get_subscribe_payload<'a>(pairs: impl AsRef<[&'a str]>) -> String {
        format!(
            r#"{{"method": "SUBSCRIBE", "params": [{}], "id": 1 }}"#,
            pairs
                .as_ref()
                .iter()
                .map(|pair| format!(r#""{pair}@bookTicker""#))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<MarketPrice> {
        match serde_json::from_str::<BinanceBookTicker>(&payload) {
            Ok(tick) => {
                let price = tick.price();
                Some(MarketPrice { exchange_id: Self::exchange_id(), market: tick.symbol, price })
            }
            Err(_) => None,
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
    use crate::websocket::websocket_run;

    use super::*;
    use env_logger::Env;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, rx) = tokio::sync::mpsc::channel(1000);

        websocket_run::<Binance>(tx, ["btcusdt"]).await;
        assert_eq!(dec!(1), dec!(1));
    }
}
