use ci::api::Storage;
use ci::eutxo::eutxo_model::*;

use std::sync::Arc;
use std::sync::RwLock;

use ci::block_service::BlockService;
use ci::eutxo::btc::btc_block_provider::BtcBlockProvider;
use ci::eutxo::eutxo_block_monitor::EuBlockMonitor;
use ci::eutxo::eutxo_families::EutxoFamilies;
use ci::eutxo::eutxo_index_manager::DbIndexManager;
use ci::eutxo::eutxo_model::EuTx;
use ci::eutxo::eutxo_tx_service::EuTxService;
use ci::indexer::Indexer;
use ci::info;
use ci::model::*;
use ci::rocks_db_batch::{Families, SharedFamilies};
use ci::settings::AppConfig;
use ci::syncer::ChainSyncer;
use ci::{api::BlockProvider, eutxo::eutxo_storage};
use rocksdb::BoundColumnFamily;

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
                    let db = Arc::new(eutxo_storage::get_db(&db_index_manager, &db_path));
                    let families = Arc::new(Families {
                        shared: SharedFamilies {
                            meta_cf: db.cf_handle(META_CF).unwrap(),
                            block_hash_by_pk_cf: db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
                            block_pk_by_hash_cf: db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
                            tx_hash_by_pk_cf: db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
                            tx_pk_by_hash_cf: db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
                        },
                        custom: EutxoFamilies {
                            utxo_value_by_pk_cf: db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
                            utxo_pk_by_input_pk_cf: db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
                            utxo_birth_pk_with_utxo_pk_cf: db_index_manager
                                .utxo_birth_pk_relations
                                .iter()
                                .map(|cf| db.cf_handle(cf).unwrap())
                                .collect::<Vec<Arc<BoundColumnFamily>>>(),
                            utxo_birth_pk_by_index_cf: db_index_manager
                                .utxo_birth_pk_by_index
                                .iter()
                                .map(|cf| db.cf_handle(cf).unwrap())
                                .collect::<Vec<Arc<BoundColumnFamily>>>(),
                            index_by_utxo_birth_pk_cf: db_index_manager
                                .index_by_utxo_birth_pk
                                .iter()
                                .map(|cf| db.cf_handle(cf).unwrap())
                                .collect::<Vec<Arc<BoundColumnFamily>>>(),
                            assets_by_utxo_pk_cf: db.cf_handle(ASSETS_BY_UTXO_PK_CF).unwrap(),
                            asset_id_by_asset_birth_pk_cf: db
                                .cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF)
                                .unwrap(),
                            asset_birth_pk_by_asset_id_cf: db
                                .cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF)
                                .unwrap(),
                            asset_birth_pk_with_asset_pk_cf: db
                                .cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF)
                                .unwrap(),
                        },
                    });

                    let storage = Arc::new(RwLock::new(Storage {
                        db: Arc::clone(&db),
                    }));

                    let tx_service = Arc::new(EuTxService {});
                    let block_service = Arc::new(BlockService::new(tx_service));

                    let block_provider: Arc<
                        dyn BlockProvider<InTx = bitcoin::Transaction, OutTx = EuTx> + Send + Sync,
                    > = Arc::new(BtcBlockProvider::new(
                        &api_host,
                        &api_username,
                        &api_password,
                    ));
                    let indexer = Arc::new(Indexer::new(
                        Arc::clone(&storage),
                        Arc::clone(&families),
                        block_service,
                        Arc::clone(&block_provider),
                    ));
                    let syncer = ChainSyncer::new(
                        Arc::clone(&block_provider),
                        Arc::new(EuBlockMonitor::new(1000)),
                        indexer,
                    );
                    let storage = Arc::clone(&storage);
                    tokio::task::spawn(async move {
                        match tokio::signal::ctrl_c().await {
                            Ok(()) => {
                                info!("Received interrupt signal");
                                storage.write().unwrap().db.flush().unwrap();
                                info!("RocksDB successfully flushed and closed.");
                                std::process::exit(0);
                            }
                            Err(err) => {
                                eprintln!("Unable to listen for shutdown signal: {}", err);
                                std::process::exit(1);
                            }
                        }
                    });
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
