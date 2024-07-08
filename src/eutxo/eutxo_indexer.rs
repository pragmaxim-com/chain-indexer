use crate::api::Block;
use crate::api::ChainLinker;
use crate::api::DbIndexName;
use crate::api::Indexer;
use crate::eutxo::eutxo_api::EuBlock;
use crate::info;
use lru::LruCache;
use rocksdb::{
    BoundColumnFamily, MultiThreaded, Options, TransactionDB, TransactionDBOptions,
    WriteBatchWithTransaction,
};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;

use super::eutxo_service;

pub const BLOCK_HASH_BY_PK_CF: &str = "BLOCK_HASH_BY_PK_CF";
pub const BLOCK_PK_BY_HASH_CF: &str = "BLOCK_PK_BY_HASH_CF";
pub const TX_HASH_BY_PK_CF: &str = "TX_HASH_BY_PK_CF";
pub const TX_PK_BY_HASH_CF: &str = "TX_PK_BY_HASH_CF";
pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const META_CF: &str = "META_CF";

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

pub struct EutxoIndexer<InBlock: Send + Sync> {
    db: Arc<TransactionDB<MultiThreaded>>,
    tx_pk_by_tx_hash_lru_cache: Mutex<LruCache<[u8; 32], [u8; 6]>>,
    utxo_indexes: Vec<DbIndexName>,
    chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = EuBlock> + Send + Sync>,
}

impl<InBlock: Block + Send + Sync> EutxoIndexer<InBlock> {
    pub fn new(
        db_path: &str,
        utxo_indexes: Vec<DbIndexName>,
        chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = EuBlock> + Send + Sync>,
    ) -> Self {
        let num_cores = num_cpus::get() as i32;
        info!("Number of CPU cores: {}", num_cores);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        // Increase parallelism: setting the number of background threads
        opts.increase_parallelism(num_cores); // Set this based on your CPU cores
        opts.set_max_background_jobs(std::cmp::max(num_cores / 2, 6));
        // Set other options for performance
        opts.set_max_file_opening_threads(std::cmp::max(num_cores, 16));
        opts.set_write_buffer_size(128 * 1024 * 1024); // 64 MB
        opts.set_max_write_buffer_number(8);
        opts.set_target_file_size_base(128 * 1024 * 1024); // 64 MB
        opts.set_max_bytes_for_level_base(512 * 1024 * 1024);
        opts.set_use_direct_io_for_flush_and_compaction(true);

        let existing_cfs =
            TransactionDB::<MultiThreaded>::list_cf(&opts, db_path).unwrap_or(vec![]);

        let txn_db_opts = TransactionDBOptions::default();

        let instance = TransactionDB::<MultiThreaded>::open_cf(
            &opts,
            &txn_db_opts,
            db_path.to_string(),
            &existing_cfs,
        )
        .unwrap();

        if existing_cfs.is_empty() {
            let options = rocksdb::Options::default();
            for cf in get_column_families().into_iter() {
                info!("Creating column family: {}", cf);
                instance.create_cf(cf, &options).unwrap();
            }
            for cf in utxo_indexes.iter() {
                info!("Creating column family: {}", cf);
                instance.create_cf(cf, &options).unwrap();
            }
        }

        EutxoIndexer {
            db: Arc::new(instance),
            tx_pk_by_tx_hash_lru_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(10_000_000).unwrap(),
            )),
            utxo_indexes,
            chain_linker,
        }
    }
}

pub fn get_column_families() -> Vec<&'static str> {
    vec![
        META_CF,
        BLOCK_HASH_BY_PK_CF,
        BLOCK_PK_BY_HASH_CF,
        TX_HASH_BY_PK_CF,
        TX_PK_BY_HASH_CF,
        UTXO_VALUE_BY_PK_CF,
        UTXO_PK_BY_INPUT_PK_CF,
    ]
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut array = [0u8; std::mem::size_of::<u32>()];
    array.copy_from_slice(bytes);
    u32::from_ne_bytes(array)
}

fn u32_to_bytes(n: u32) -> [u8; std::mem::size_of::<u32>()] {
    n.to_ne_bytes()
}

// implement BlockBatchIndexer trait
impl<InBlock: Send + Sync> Indexer for EutxoIndexer<InBlock> {
    type OutBlock = EuBlock;

    fn get_last_height(&self) -> u32 {
        let meta_cf = self.db.cf_handle(META_CF).unwrap();
        self.db
            .get_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY)
            .unwrap()
            .map_or(0, |height| bytes_to_u32(&height))
    }

    fn consume(&self, blocks: &Vec<EuBlock>) -> Result<(), String> {
        let block_hash_by_pk_cf = self.db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap();
        let block_pk_by_hash_cf = self.db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap();
        let tx_hash_by_pk_cf = self.db.cf_handle(TX_HASH_BY_PK_CF).unwrap();
        let tx_pk_by_hash_cf = self.db.cf_handle(TX_PK_BY_HASH_CF).unwrap();
        let utxo_value_by_pk_cf = self.db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap();
        let utxo_pk_by_input_pk_cf = self.db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap();
        let meta_cf = self.db.cf_handle(META_CF).unwrap();

        let index_cf_by_name: &Vec<(DbIndexName, Arc<BoundColumnFamily>)> = &self
            .utxo_indexes
            .iter()
            .map(|index_name| (index_name.clone(), self.db.cf_handle(&index_name).unwrap()))
            .collect();

        let db_tx = self.db.transaction();
        let mut batch: WriteBatchWithTransaction<true> = db_tx.get_writebatch();

        let mut tx_pk_by_tx_hash_lru_cache = self
            .tx_pk_by_tx_hash_lru_cache
            .lock()
            .map_err(|e| e.to_string())?;

        for block in blocks.iter() {
            eutxo_service::process_block(
                &block.height,
                &block.hash,
                &db_tx,
                &mut batch,
                &block_hash_by_pk_cf,
                &block_pk_by_hash_cf,
            )
            .map_err(|e| e.into_string())?;
            for eu_tx in block.txs.iter() {
                eutxo_service::process_tx(
                    &block.height,
                    eu_tx,
                    &db_tx,
                    &mut batch,
                    &tx_hash_by_pk_cf,
                    &tx_pk_by_hash_cf,
                    &mut tx_pk_by_tx_hash_lru_cache,
                )
                .map_err(|e| e.into_string())?;
                eutxo_service::process_outputs(
                    &block.height,
                    eu_tx,
                    &mut batch,
                    &utxo_value_by_pk_cf,
                    index_cf_by_name,
                );
                if !eu_tx.is_coinbase {
                    eutxo_service::process_inputs(
                        block.height,
                        eu_tx,
                        &db_tx,
                        &mut batch,
                        &utxo_pk_by_input_pk_cf,
                        &tx_pk_by_hash_cf,
                        &mut tx_pk_by_tx_hash_lru_cache,
                    );
                }
            }
        }
        // persist last height to db_tx if Some
        if let Some(block) = blocks.last() {
            db_tx
                .put_cf(
                    &meta_cf,
                    LAST_ADDRESS_HEIGHT_KEY,
                    u32_to_bytes(block.height),
                )
                .map_err(|e| e.into_string())?;
        }
        db_tx.commit().map_err(|e| e.into_string())?;
        Ok(())
    }
}
