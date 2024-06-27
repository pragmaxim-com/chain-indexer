mod api;
mod btc;
mod codec;
mod config;
mod logger;
mod syncer;

use api::ChainSyncer;
use btc::{btc_client::BtcClient, btc_processor::BtcProcessor, btc_storage::BtcStorage};
use config::AppConfig;
use std::{env, sync::Arc};

fn enforce_env_var(name: &str) {
    if env::var(name).is_err() {
        panic!("Environment variable {} is not set", name);
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new();

    match config {
        Ok(config) => {
            config.print();
            let blockchain_settings = config.blockchain;
            let db_path = blockchain_settings.db_path;
            let api_host = blockchain_settings.api_host;
            let api_username = blockchain_settings.api_username;
            let api_password = blockchain_settings.api_password;
            let name = blockchain_settings.name;
            let num_cores = num_cpus::get();

            log!("Number of CPU cores: {}", num_cores);

            let full_db_path = format!("{}/{}", db_path, name);

            let client = Arc::new(BtcClient::new(&api_host, &api_username, &api_password));
            let processor = Arc::new(BtcProcessor {});
            let storage = BtcStorage::new(num_cores as i32, &full_db_path).unwrap();
            let syncer = ChainSyncer::new(client, processor, Arc::new(storage));
            syncer.sync(844566, 50).await;
            Ok(())
        }
        Err(e) => {
            log!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
