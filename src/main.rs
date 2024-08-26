use env_logger::Env;

mod exchange;
mod engine;
mod websocket;

pub type Sender<T> = tokio::sync::mpsc::Sender<T>;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Hello, world!");
}
