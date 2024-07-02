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
            let db_path = format!("{}/{}", blockchain.db_path, blockchain.name);
            let db_indexes = config.indexer.db_indexes;
            let tx_batch_size = config.indexer.tx_batch_size;

            match blockchain.name.as_str() {
                "btc" => {
                    ChainSyncer::new(
                        Arc::new(BtcClient::new(&api_host, &api_username, &api_password)),
                        Arc::new(BtcProcessor {}),
                        Arc::new(EutxoIndexers::new(&db_path, db_indexes)),
                    )
                    .sync(844566, tx_batch_size)
                    .await;
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
