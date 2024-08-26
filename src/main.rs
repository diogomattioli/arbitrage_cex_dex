use env_logger::Env;

mod engine;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    log::info!("Hello, world!");
}
