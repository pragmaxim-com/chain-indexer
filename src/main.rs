use ci::api::ChainLinker;
use ci::block_service::BlockService;
use ci::db_index_manager::DbIndexManager;
use ci::eutxo::btc::btc_chain_linker::BtcChainLinker;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
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
            let db_utxo_indexes = config.indexer.db_indexes;

            let tx_batch_size = config.indexer.tx_batch_size;

            match blockchain.name.as_str() {
                "btc" => {
                    let db_index_manager = Arc::new(DbIndexManager::new(&db_utxo_indexes));
                    let db_holder = Arc::new(Storage::new(
                        &db_path,
                        Arc::clone(&db_index_manager),
                        eutxo_model::get_eutxo_column_families(),
                    ));
                    // let db_holder = Arc::new(DbHolder { db: Arc::new(db) });
                    let tx_service: Arc<EuTxService> = Arc::new(EuTxService {
                        db_index_manager: Arc::clone(&db_index_manager),
                    });
                    let block_service: Arc<BlockService<EuTx>> =
                        Arc::new(BlockService::new(tx_service));

                    let chain_linker: Arc<
                        dyn ChainLinker<InTx = bitcoin::Transaction, OutTx = EuTx> + Send + Sync,
                    > = Arc::new(BtcChainLinker::new(&api_host, &api_username, &api_password));
                    let syncer = ChainSyncer::new(
                        Arc::clone(&chain_linker),
                        Arc::new(EuBlockMonitor::new(1000)),
                        Arc::new(Indexer::new(
                            db_holder,
                            block_service,
                            Arc::clone(&chain_linker),
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
