use crate::api::Storage;
use crate::cli::CliConfig;
use crate::eutxo::eutxo_model::*;
use crate::settings::IndexerSettings;

use std::sync::Arc;
use std::sync::RwLock;

use crate::block_service::BlockService;
use crate::eutxo::eutxo_block_monitor::EuBlockMonitor;
use crate::eutxo::eutxo_families::EutxoFamilies;
use crate::eutxo::eutxo_model::EuTx;
use crate::eutxo::eutxo_tx_service::EuTxService;
use crate::indexer::Indexer;
use crate::info;
use crate::model::*;
use crate::rocks_db_batch::{Families, SharedFamilies};
use crate::syncer::ChainSyncer;
use crate::{api::BlockProvider, eutxo::eutxo_storage};
use rocksdb::BoundColumnFamily;

pub async fn run_eutxo_indexing(
    indexer_settings: IndexerSettings,
    cli_config: CliConfig,
    block_provider: Arc<dyn BlockProvider<OutTx = EuTx>>,
) {
    let db_path: String = format!(
        "{}/{}/{}",
        indexer_settings.db_path, "main", cli_config.blockchain
    );
    let disable_wal = indexer_settings.disable_wal;

    let tx_batch_size = indexer_settings.tx_batch_size;
    let db_index_manager = block_provider.get_index_manager();
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
            input_pk_by_utxo_pk_cf: db.cf_handle(INPUT_PK_BY_UTXO_PK_CF).unwrap(),
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
            asset_by_asset_pk_cf: db.cf_handle(ASSET_BY_ASSET_PK_CF).unwrap(),
            asset_id_by_asset_birth_pk_cf: db.cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF).unwrap(),
            asset_birth_pk_by_asset_id_cf: db.cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF).unwrap(),
            asset_birth_pk_with_asset_pk_cf: db.cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF).unwrap(),
        },
    });

    let storage = Arc::new(RwLock::new(Storage {
        db: Arc::clone(&db),
    }));

    let tx_service = Arc::new(EuTxService {});
    let block_service = Arc::new(BlockService::new(tx_service));

    let indexer = Arc::new(Indexer::new(
        Arc::clone(&storage),
        Arc::clone(&families),
        block_service,
        Arc::clone(&block_provider) as Arc<dyn BlockProvider<OutTx = EuTx>>,
        disable_wal,
    ));
    let syncer = ChainSyncer::new(block_provider, Arc::new(EuBlockMonitor::new(1000)), indexer);
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
    syncer.sync(tx_batch_size).await
}
