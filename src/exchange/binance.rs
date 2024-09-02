use rust_decimal_macros::dec;
use serde::Deserialize;

use rust_decimal::Decimal;

use crate::{ websocket::ExchangeWebSocketConfig, MarketPrice };

pub struct Binance;

impl ExchangeWebSocketConfig for Binance {
    fn exchange_id() -> &'static str {
        "binance"
    }

    fn url() -> String {
        "wss://stream.binance.com:9443/ws".to_string()
    }

    fn get_subscribe_payload(markets: &[&str]) -> String {
        format!(
            r#"{{"method": "SUBSCRIBE", "params": [{}], "id": 1 }}"#,
            markets
                .as_ref()
                .iter()
                .map(|market| format!(r#""{market}@bookTicker""#))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<MarketPrice> {
        let tick = serde_json::from_str::<BinanceBookTicker>(&payload).ok()?;
        let price = tick.price();

        Some(MarketPrice { exchange_id: Self::exchange_id(), market: tick.symbol, price })
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
    use crate::websocket::run_websocket;

    use super::*;
    use env_logger::Env;

    #[ignore]
    #[tokio::test]
    async fn test_run() {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

        let (tx, _rx) = tokio::sync::watch::channel(MarketPrice::default());

        run_websocket::<Binance>(tx, &["btcusdt"]).await;
    }

    #[test]
    fn test_get_subscribe_payload() {
        let payload = Binance::get_subscribe_payload(&["btcusdt", "ethusdt"]);
        assert_eq!(
            payload,
            r#"{"method": "SUBSCRIBE", "params": ["btcusdt@bookTicker", "ethusdt@bookTicker"], "id": 1 }"#
        );
    }

    #[test]
    fn test_parse_incoming_payload() {
        let payload =
            r#"{
                "e": "24hrTicker",  
                "E": 1672515782136, 
                "s": "BNBBTC",      
                "p": "0.0015",      
                "P": "250.00",      
                "w": "0.0018",      
                "x": "0.0009",      
                "c": "0.0025",      
                "Q": "10",          
                "b": "0.0024",      
                "B": "10",          
                "a": "0.0026",      
                "A": "100",         
                "o": "0.0010",      
                "h": "0.0025",      
                "l": "0.0010",      
                "v": "10000",       
                "q": "18",          
                "O": 0,             
                "C": 86400000,      
                "F": 0,             
                "L": 18150,         
                "n": 18151          
            }"#;
        let market_price = Binance::parse_incoming_payload(payload.to_string()).unwrap();
        assert_eq!(market_price.market, "BNBBTC");
        assert_eq!(market_price.price, dec!(0.0025));
    }
}
