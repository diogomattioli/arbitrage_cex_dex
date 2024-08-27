use env_logger::Env;
use rust_decimal::Decimal;
use tokio::join;
use websocket::websocket_run;

mod exchange;
mod engine;
mod websocket;

use exchange::{ binance::Binance, kraken::Kraken };

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

    let mut sigterm = tokio::signal::unix
        ::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("cannot listen for sigterm");

    let engine_run = async move {
        let mut engine = engine::Engine::<&str>::default();

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = sigterm.recv() => break,     

                r = rx.changed() => {
                    if r.is_err() {
                        break;
                    }

                    let market_price = rx.borrow_and_update().clone();

                    log::trace!("{market_price:?}");

                    engine.update(market_price.exchange_id, market_price.price);
                },   
            }
        }
    };

    join!(
        engine_run,
        websocket_run::<Binance>(tx.clone(), ["btcusdt"]),
        websocket_run::<Kraken>(tx, ["BTC/USDT"])
    );

    log::info!("gracefully exiting!");
}
