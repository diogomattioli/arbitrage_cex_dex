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

    fn get_subscribe_payload<'a>(markets: &[&'a str]) -> String {
        format!(
            r#"{{"method": "subscribe", "params": {{"channel": "ticker", "snapshot": false, "event_trigger": "bbo", "symbol": [{}]}}, "req_id": 1 }}"#,
            markets
                .as_ref()
                .iter()
                .map(|market| format!(r#""{market}""#))
                .collect::<Vec<_>>()
                .join(", ")
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
    use crate::websocket::run_websocket;

    use super::*;
    use env_logger::Env;

    #[ignore]
    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, _rx) = tokio::sync::watch::channel(MarketPrice::default());

        run_websocket::<Kraken>(tx, &["BTC/USDT"]).await;
    }

    #[test]
    fn test_get_subscribe_payload() {
        let payload = Kraken::get_subscribe_payload(&["BTC/USDT", "ETH/USDT"]);
        assert_eq!(
            payload,
            r#"{"method": "subscribe", "params": {"channel": "ticker", "snapshot": false, "event_trigger": "bbo", "symbol": ["BTC/USDT", "ETH/USDT"]}, "req_id": 1 }"#
        );
    }

    #[test]
    fn test_parse_incoming_payload() {
        let payload =
            r#"{
                "channel": "ticker",
                "type": "snapshot",
                "data": [
                    {
                        "symbol": "ALGO/USD",
                        "bid": 0.10025,
                        "bid_qty": 740.0,
                        "ask": 0.10036,
                        "ask_qty": 1361.44813783,
                        "last": 0.10035,
                        "volume": 997038.98383185,
                        "vwap": 0.10148,
                        "low": 0.09979,
                        "high": 0.10285,
                        "change": -0.00017,
                        "change_pct": -0.17
                    }
                ]
            }"#;
        let market_price = Kraken::parse_incoming_payload(payload.to_string()).unwrap();
        assert_eq!(market_price.market, "ALGO/USD");
        assert_eq!(market_price.price, dec!(0.100305));
    }
}
