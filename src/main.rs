use ci::api::ChainLinker;
use ci::eutxo::btc::btc_chain_linker::BtcChainLinker;
use ci::eutxo::btc::btc_client::BtcBlock;
use ci::eutxo::btc::{btc_client::BtcClient, btc_processor::BtcProcessor};
use ci::eutxo::eutxo_api::EuBlock;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
use ci::eutxo::eutxo_service::EuService;
use ci::indexer::{self, Indexer};
use ci::rocksdb_wrapper::RocksDbWrapper;
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
                    let db = Arc::new(RocksDbWrapper::initiate(&db_path, db_indexes));
                    let service: Arc<EuService> = Arc::new(EuService::new(Arc::clone(&db)));
                    let chain_linker: Arc<
                        dyn ChainLinker<InBlock = BtcBlock, OutBlock = EuBlock> + Send + Sync,
                    > = Arc::new(BtcChainLinker {
                        client: BtcClient::new(&api_host, &api_username, &api_password),
                        processor: BtcProcessor {},
                    });
                    let syncer = ChainSyncer::new(
                        Arc::clone(&chain_linker),
                        Arc::new(EuBlockMonitor::new(1000)),
                        Arc::new(Indexer::new(
                            Arc::clone(&db),
                            service,
                            Arc::clone(&chain_linker),
                        )),
                    );
                    // syncer.setup_signal_handler();
                    syncer.sync(tx_batch_size).await;
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
