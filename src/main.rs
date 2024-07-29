use ci::api::Storage;
use ci::block_service::BlockService;
use ci::eutxo::btc::btc_block_provider::BtcBlockProvider;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
use ci::eutxo::eutxo_index_manager::DbIndexManager;
use ci::eutxo::eutxo_model::EuTx;
use ci::eutxo::eutxo_tx_service::EuTxService;
use ci::indexer::Indexer;
use ci::settings::AppConfig;
use ci::syncer::ChainSyncer;
use ci::{api::BlockProvider, eutxo::eutxo_storage};
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
            let db_path: String = format!("{}/{}/{}", blockchain.db_path, "main", blockchain.name);
            let db_indexes = config.indexer.db_indexes;

            let tx_batch_size = config.indexer.tx_batch_size;

            match blockchain.name.as_str() {
                "btc" => {
                    let db_index_manager = DbIndexManager::new(&db_indexes);
                    let db = eutxo_storage::get_db(&db_index_manager, &db_path);
                    let families = eutxo_storage::get_families(&db_index_manager, &db);
                    let storage = Storage {
                        db: &db,
                        families: &families,
                    };
                    let tx_service: Arc<EuTxService> = Arc::new(EuTxService {});
                    let block_service = Arc::new(BlockService::new(tx_service));

                    let block_provider: Arc<
                        dyn BlockProvider<InTx = bitcoin::Transaction, OutTx = EuTx> + Send + Sync,
                    > = Arc::new(BtcBlockProvider::new(
                        &api_host,
                        &api_username,
                        &api_password,
                    ));
                    let syncer = ChainSyncer::new(
                        Arc::clone(&block_provider),
                        Arc::new(EuBlockMonitor::new(1000)),
                        Arc::new(Indexer::new(
                            &storage,
                            block_service,
                            Arc::clone(&block_provider),
                        )),
                    );
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
