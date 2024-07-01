mod api;
mod config;
mod eutxo;
mod logger;
mod syncer;

use api::ChainSyncer;
use btc::{btc_client::BtcClient, btc_processor::BtcProcessor};
use config::AppConfig;
use eutxo::btc;
use eutxo::eutxo_indexers::EutxoIndexers;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new();

    match config {
        Ok(config) => {
            let blockchain = config.blockchain;
            let api_host = blockchain.api_host;
            let api_username = blockchain.api_username;
            let api_password = blockchain.api_password;
            let full_db_path = format!("{}/{}", blockchain.db_path, blockchain.name);

            match blockchain.name.as_str() {
                "btc" => {
                    let client = Arc::new(BtcClient::new(&api_host, &api_username, &api_password));
                    let processor = Arc::new(BtcProcessor {});
                    let storage =
                        EutxoIndexers::new(&full_db_path, config.indexer.utxo_indexes).unwrap();
                    let syncer = ChainSyncer::new(client, processor, Arc::new(storage));
                    syncer.sync(844566, 1000).await;
                    Ok(())
                }
                _ => {
                    error!("Unsupported blockchain: {}", blockchain.name);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"));
                }
            }
        }
        Err(e) => {
            error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
