use btc::{btc_client::BtcClient, btc_processor::BtcProcessor};
use ci::api::ChainLinker;
use ci::eutxo::btc;
use ci::eutxo::btc::btc_chain_linker::BtcChainLinker;
use ci::eutxo::btc::btc_client::BtcBlock;
use ci::eutxo::eutxo_api::EuBlock;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
use ci::eutxo::eutxo_indexer::EutxoIndexer;
use ci::settings::AppConfig;
use ci::syncer::ChainSyncer;
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
            let db_path = format!("{}/{}/{}", blockchain.db_path, "main", blockchain.name);
            let db_indexes = config.indexer.db_indexes;
            let tx_batch_size = config.indexer.tx_batch_size;

            match blockchain.name.as_str() {
                "btc" => {
                    let chain_linker: Arc<
                        dyn ChainLinker<InBlock = BtcBlock, OutBlock = EuBlock> + Send + Sync,
                    > = Arc::new(BtcChainLinker {
                        client: BtcClient::new(&api_host, &api_username, &api_password),
                        processor: BtcProcessor {},
                    });
                    ChainSyncer::new(
                        Arc::clone(&chain_linker),
                        Arc::new(EuBlockMonitor::new(1000)),
                        Arc::new(EutxoIndexer::new(
                            &db_path,
                            db_indexes,
                            Arc::clone(&chain_linker),
                        )),
                    )
                    .sync(tx_batch_size)
                    .await;
                    Ok(())
                }
                _ => {
                    ci::error!("Unsupported blockchain: {}", blockchain.name);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"));
                }
            }
        }
        Err(e) => {
            ci::error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
