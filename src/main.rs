use ci::api::{BatchExecutor, BlockProvider};
use ci::block_service::BlockService;
use ci::db_options;
use ci::eutxo::btc::btc_block_provider::BtcBlockProvider;
use ci::eutxo::eutxo_batch_executor::EutxoBatchExecutor;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
use ci::eutxo::eutxo_index_manager::DbIndexManager;
use ci::eutxo::eutxo_model::{self, EuTx};
use ci::eutxo::eutxo_tx_service::EuTxService;
use ci::indexer::Indexer;
use ci::settings::AppConfig;
use ci::storage::Storage;
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
            let db_path: String = format!("{}/{}/{}", blockchain.db_path, "main", blockchain.name);
            let db_indexes = config.indexer.db_indexes;

            let tx_batch_size = config.indexer.tx_batch_size;

            match blockchain.name.as_str() {
                "btc" => {
                    let db_index_manager = Arc::new(DbIndexManager::new(&db_indexes));
                    let options: db_options::get_db_options();
                    let existing_cfs =
                        OptimisticTransactionDB::<SingleThreaded>::list_cf(&options, db_path)
                            .unwrap_or(vec![]);

                    let mut db = OptimisticTransactionDB::<SingleThreaded>::open_cf(
                        &options,
                        db_path,
                        &existing_cfs,
                    )
                    .unwrap();

                    let batch_executor = Arc::new(EutxoBatchExecutor::new(
                        &mut db,
                        options,
                        db_indexes,
                        existing_cfs,
                    ));
                    let tx_service: Arc<EuTxService> = Arc::new(EuTxService {
                        db_index_manager: Arc::clone(&db_index_manager),
                    });
                    let block_service: Arc<BlockService<EuTx>> =
                        Arc::new(BlockService::new(tx_service));

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
                            Arc::clone(&batch_executor.db),
                            batch_executor,
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
