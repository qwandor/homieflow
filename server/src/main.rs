use houseflow_config::{defaults, server::Config, Config as _};
use houseflow_db::sqlite::Database as SqliteDatabase;
use houseflow_server::{Sessions, SledTokenStore};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const HIDE_TIMESTAMP_ENV: &str = "HOUSEFLOW_SERVER_HIDE_TIMESTAMP";

    houseflow_config::init_logging(std::env::var_os(HIDE_TIMESTAMP_ENV).is_some());
    let config_path = std::env::var("HOUSEFLOW_SERVER_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Config::default_path());

    let config = Config::read(config_path).expect("cannot load server config");
    let token_store = SledTokenStore::new(&config.tokens_path).expect("cannot open token store");
    let database = SqliteDatabase::new(&config.database_path).expect("cannot open database");
    let sessions = Sessions::new();

    let state = houseflow_server::State {
        token_store: Arc::new(token_store),
        database: Arc::new(database),
        config: Arc::new(config),
        sessions: Arc::new(Mutex::new(sessions)),
    };

    let address = format!("{}:{}", state.config.hostname, defaults::server_port());
    let socket_address: std::net::SocketAddr = address.parse().expect("invalid address");
    houseflow_server::run(&socket_address, state).await;

    Ok(())
}
