use env_logger::Env;
use rust_decimal::Decimal;
use tokio::join;
use websocket::websocket_run;

mod exchange;
mod engine;
mod websocket;

use exchange::binance::Binance;

pub type Sender<T> = tokio::sync::watch::Sender<T>;

#[derive(Default, Debug, Clone)]
struct MarketPrice {
    exchange_id: &'static str,
    market: String,
    price: Decimal,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let (tx, mut rx) = tokio::sync::watch::channel(MarketPrice::default());

    let engine_fut = async move {
        let mut engine = engine::Engine::<&str>::default();

        while rx.changed().await.is_ok() {
            let market_price = rx.borrow_and_update().clone();

            log::trace!("{market_price:?}");

            engine.update(market_price.exchange_id, market_price.price);

            engine.iter().for_each(|(exchange_id, price)| {
                log::info!("iter {exchange_id}: {:?}", price.map(|x| *x).collect::<Vec<_>>());
            });
        }
    };

    join!(engine_fut, websocket_run::<Binance>(tx, ["btcusdt"]));
}
