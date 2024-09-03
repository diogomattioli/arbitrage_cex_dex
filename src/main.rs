use env_logger::Env;
use futures::future::select_all;
use rust_decimal::Decimal;
use tokio::{ join, sync::watch::{ error::RecvError, Receiver } };
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

    let (tx, mut rx_binance) = tokio::sync::watch::channel(MarketPrice::default());
    let future_binance = run_websocket::<Binance>(tx, &["solusdt"]);

    let (tx, mut rx_kraken) = tokio::sync::watch::channel(MarketPrice::default());
    let future_kraken = run_websocket::<Kraken>(tx, &["SOL/USDT"]);

    let (tx, mut rx_helius) = tokio::sync::watch::channel(MarketPrice::default());
    let future_helius = run_websocket::<Helius>(
        tx,
        &["3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF"]
    );

    let future_engine = async move {
        let mut engine = engine::Engine::<&str>::default();

        let mut sigterm = tokio::signal::unix
            ::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("cannot listen for sigterm");

        loop {
            async fn future_receiver_changed(
                rx: &mut Receiver<MarketPrice>
            ) -> (Result<(), RecvError>, &mut Receiver<MarketPrice>) {
                (rx.changed().await, rx)
            }

            let receivers_changed = [
                Box::pin(future_receiver_changed(&mut rx_binance)),
                Box::pin(future_receiver_changed(&mut rx_kraken)),
                Box::pin(future_receiver_changed(&mut rx_helius)),
            ];

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::warn!("ctrl+c received");
                    break;
                },
                _ = sigterm.recv() => {
                    log::warn!("sigterm received");
                    break;
                },     

                ((res, rx), ..) = select_all(receivers_changed) => {
                    if res.is_err() {
                        // sender is closed
                        break;
                    }

                    let market_price = rx.borrow_and_update().clone();

                    log::trace!("{market_price:?}");

                    engine.update(market_price.exchange_id, market_price.price);

                    // print prices list
                    log::info!("");
                    log::info!("Prices:");

                    engine
                        .iter()
                        .enumerate()
                        .for_each(|(idx, (price, exchange_ids))| {
                            let mut price = *price;
                            price.rescale(4);

                            log::info!(
                                "   {} - {price} {:?}",
                                idx + 1,
                                exchange_ids.map(|id| *id).collect::<Vec<_>>()
                            );
                        });
                }
            }
        }
    };

    join!(future_engine, future_binance, future_kraken, future_helius);

    log::info!("gracefully exiting!");
}
