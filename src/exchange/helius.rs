use borsh::BorshDeserialize;
use rust_decimal::{ prelude::FromPrimitive, Decimal };
use rust_decimal_macros::dec;
use serde::Deserialize;

use base64::prelude::*;

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

    fn get_subscribe_payload(markets: &[&str]) -> String {
        format!(
            r#"{{"jsonrpc": "2.0", "method": "accountSubscribe", "params": [{}, {{"encoding": "base64", "commitment": "confirmed"}}], "id": 1 }}"#,
            markets
                .as_ref()
                .iter()
                .map(|market| format!(r#""{market}""#))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn parse_incoming_payload(payload: String) -> Option<MarketPrice> {
        match serde_json::from_str::<HeliusEnvelope>(&payload) {
            Ok(envelope) => {
                let owner = envelope.params.result.value.owner.clone();
                let pool_state: PoolState = envelope.into();
                let price = pool_state.price();

                Some(MarketPrice {
                    exchange_id: Self::exchange_id(),
                    market: owner,
                    price,
                })
            }
            Err(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct HeliusEnvelope {
    params: HeliusParams,
}

#[derive(Deserialize, Debug)]
struct HeliusParams {
    result: HeliusResult,
}

#[derive(Deserialize, Debug)]
struct HeliusResult {
    value: HeliusValue,
}

#[derive(Deserialize, Debug)]
struct HeliusValue {
    owner: String,
    data: HeliusData,
}

#[derive(Deserialize, Debug)]
struct HeliusData(Vec<String>);

#[repr(C)]
#[derive(BorshDeserialize)]
struct PoolState {
    bump: u8,
    padding_before: [u8; 252],
    sqrt_price_x64: u128,
    padding_after: [u8; 1275],
}

impl PoolState {
    pub fn price(&self) -> Decimal {
        let price_x64 = self.sqrt_price_x64.pow(2);
        let pre_normalized = price_x64 / (2_u128).pow(64);
        let normalized =
            Decimal::from_u128(pre_normalized).unwrap_or(dec!(0)) /
            Decimal::from_u128((2_u128).pow(64)).unwrap_or(dec!(1));
        let price = normalized * dec!(1000);
        price
    }
}

impl From<HeliusEnvelope> for PoolState {
    fn from(envelope: HeliusEnvelope) -> Self {
        let base64 = envelope.params.result.value.data.0[0].clone();
        let decoded = BASE64_STANDARD.decode(base64).unwrap();
        PoolState::try_from_slice(&decoded).unwrap()
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
        run_websocket::<Helius>(tx, &["3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF"]).await;
    }

    #[test]
    fn test_price() {
        let pool = PoolState {
            sqrt_price_x64: 6_990_823_775_062_275_942,
            bump: 0,
            padding_before: [0; 252],
            padding_after: [0; 1275],
        };
        assert_eq!(pool.price().round_dp(2), dec!(143.62));
    }

    #[test]
    fn test_decoding() {
        let envelope = HeliusEnvelope {
            params: HeliusParams {
                result: HeliusResult {
                    value: HeliusValue {
                        owner: "3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF".to_string(),
                        data: HeliusData(
                            vec![
                                "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".to_string()
                            ]
                        ),
                    },
                },
            },
        };

        let pool: PoolState = envelope.into();

        assert_eq!(pool.price().round_dp(2), dec!(145.03));
    }

    #[test]
    fn test_get_subscribe_payload() {
        let payload = Helius::get_subscribe_payload(
            &["So11111111111111111111111111111111111111112"]
        );
        assert_eq!(
            payload,
            r#"{"jsonrpc": "2.0", "method": "accountSubscribe", "params": ["So11111111111111111111111111111111111111112", {"encoding": "base64", "commitment": "confirmed"}], "id": 1 }"#
        );
    }
}
