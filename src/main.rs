use env_logger::Env;
use rust_decimal::Decimal;
use tokio::join;
use websocket::run_websocket;

mod exchange;
mod engine;
mod websocket;

use exchange::{ binance::Binance, kraken::Kraken, helius::Helius };

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

    let run_engine = async move {
        let mut engine = engine::Engine::<&str>::default();

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break,
                _ = sigterm.recv() => break,     

                r = rx.changed() => {
                    if r.is_err() {
                        // all senders are closed
                        break;
                    }

                    let market_price = rx.borrow_and_update().clone();

                    log::trace!("{market_price:?}");

                    engine.update(market_price.exchange_id, market_price.price);

                    // print prices list
                    println!();
                    log::info!("Prices:");

                    engine
                        .iter()
                        .enumerate()
                        .for_each(|(idx, (price, exchange_ids))| {
                            let mut price = *price;
                            price.rescale(2);

                            log::info!(
                                "   {} - {price} {:?}",
                                idx + 1,
                                exchange_ids.map(|id| *id).collect::<Vec<_>>()
                            );
                        });
                },   
            }
        }
    };

    join!(
        run_engine,
        run_websocket::<Binance>(tx.clone(), ["btcusdt"]),
        run_websocket::<Kraken>(tx.clone(), ["BTC/USDT"]),
        run_websocket::<Helius>(tx, ["So11111111111111111111111111111111111111112"])
    );

    log::info!("gracefully exiting!");
}
