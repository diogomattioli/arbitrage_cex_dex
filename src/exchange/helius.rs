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
                let base64 = envelope.params.result.value.data.0[0].clone();

                let mut decoded = BASE64_STANDARD.decode(base64).ok()?;
                // must skip first 8 bytes
                decoded.drain(0..8);

                let pool = PoolState::try_from_slice(&decoded).ok()?;

                let price = pool.price();
                Some(MarketPrice {
                    exchange_id: Self::exchange_id(),
                    market: envelope.params.result.value.owner,
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
#[derive(Debug, Clone, BorshDeserialize, Default)]
struct PublicKey([u8; 32]);

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, Default)]
struct RewardInfo {
    reward_state: u8,
    open_time: u64,
    end_time: u64,
    last_update_time: u64,
    emissions_per_second_x64: u128,
    reward_total_emissioned: u64,
    reward_claimed: u64,
    token_mint: PublicKey,
    token_vault: PublicKey,
    authority: PublicKey,
    reward_growth_global_x64: u128,
}

#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, Default)]
struct PoolState {
    bump: [u8; 1],
    amm_config: PublicKey,
    owner: PublicKey,
    token_mint0: PublicKey,
    token_mint1: PublicKey,
    token_vault0: PublicKey,
    token_vault1: PublicKey,
    observation_key: PublicKey,
    mint_decimals0: u8,
    mint_decimals1: u8,
    tick_spacing: u16,
    liquidity: u128,
    sqrt_price_x64: u128,
    tick_current: i32,
    padding1: u16,
    padding2: u16,
    fee_growth_global0_x64: u128,
    fee_growth_global1_x64: u128,
    protocol_fees_token0: u64,
    protocol_fees_token1: u64,
    swap_in_amount_token0: u128,
    swap_out_amount_token1: u128,
    swap_in_amount_token1: u128,
    swap_out_amount_token0: u128,
    status: u8,
    padding: [u8; 7],
    reward_infos: [RewardInfo; 3],
    tick_array_bitmap: [u64; 16],
    total_fees_token0: u64,
    total_fees_claimed_token0: u64,
    total_fees_token1: u64,
    total_fees_claimed_token1: u64,
    fund_fees_token0: u64,
    fund_fees_token1: u64,
    open_time: u64,
    recent_epoch: u64,
    padding3: [u64; 24],
    padding4: [u64; 32],
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
            ..Default::default()
        };
        assert_eq!(pool.price().round_dp(2), dec!(143.62));
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
