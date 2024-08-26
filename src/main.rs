use env_logger::Env;
use rust_decimal::Decimal;

mod exchange;
mod engine;
mod websocket;

pub type Sender<T> = tokio::sync::mpsc::Sender<T>;

struct MarketPrice {
    exchange_id: &'static str,
    market: String,
    price: Decimal,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Hello, world!");
}
