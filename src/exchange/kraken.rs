use rust_decimal_macros::dec;
use serde::Deserialize;

use rust_decimal::Decimal;

use crate::{ websocket::ExchangeWebSocketConfig, MarketPrice };

pub struct Kraken;

impl ExchangeWebSocketConfig for Kraken {
    fn exchange_id() -> &'static str {
        "kraken"
    }

    fn url() -> String {
        "wss://ws.kraken.com/v2".to_string()
    }

    fn get_subscribe_payload<'a>(pairs: impl AsRef<[&'a str]>) -> String {
        format!(
            r#"{{"method": "subscribe", "params": {{"channel": "ticker", "snapshot": false, "event_trigger": "bbo", "symbol": [{}]}}, "req_id": 1 }}"#,
            pairs
                .as_ref()
                .iter()
                .map(|pair| format!(r#""{pair}""#))
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<MarketPrice> {
        match serde_json::from_str::<KrakenBookEnvelope>(&payload) {
            Ok(envelope) => {
                let Some(tick) = envelope.data.first() else {
                    return None;
                };
                let price = tick.price();
                Some(MarketPrice {
                    exchange_id: Self::exchange_id(),
                    market: tick.symbol.clone(),
                    price,
                })
            }
            Err(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct KrakenBookEnvelope {
    data: Vec<KrakenBookTicker>,
}

#[derive(Deserialize, Debug)]
struct KrakenBookTicker {
    symbol: String,
    bid: Decimal,
    ask: Decimal,
}

impl KrakenBookTicker {
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

        let (tx, rx) = tokio::sync::watch::channel(MarketPrice::default());

        websocket_run::<Kraken>(tx, ["BTC/USDT"]).await;
        assert_eq!(dec!(1), dec!(1));
    }
}
