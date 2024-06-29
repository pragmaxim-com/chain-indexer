use broadcast_sink::Consumer;
use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB, WriteBatchWithTransaction};
use std::{num::NonZeroUsize, sync::Arc};

use crate::eutxo::eutxo_api::{CiBlock, CiTx};

pub const ADDRESS_CF: &str = "ADDRESS_CF";
pub const CACHE_CF: &str = "CACHE_CF";
pub const META_CF: &str = "META_CF";

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

fn u32_to_bytes(n: u32) -> [u8; std::mem::size_of::<u32>()] {
    n.to_ne_bytes()
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    let mut array = [0u8; std::mem::size_of::<u32>()];
    array.copy_from_slice(bytes);
    u32::from_ne_bytes(array)
}

// Method to process the outputs of a transaction
fn process_outputs(
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    address_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache_cf: &Arc<rocksdb::BoundColumnFamily>,
    address_by_hash_lru_cache: &mut LruCache<Vec<u8>, Vec<u8>>,
    hash_by_tx_lru_cache: &mut LruCache<Vec<u8>, Vec<u8>>,
) {
    for utxo in tx.outs.iter() {
        // ...
    }
}

// Method to process the inputs of a transaction
fn process_inputs(
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    address_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache_cf: &Arc<rocksdb::BoundColumnFamily>,
    address_by_hash_lru_cache: &mut LruCache<Vec<u8>, Vec<u8>>,
    hash_by_tx_lru_cache: &mut LruCache<Vec<u8>, Vec<u8>>,
) {
    for indexed_txid in &tx.ins {
        //...
    }
}

pub fn get_column_families() -> Vec<&'static str> {
    vec![ADDRESS_CF, CACHE_CF, META_CF]
}

pub fn get_last_height(db: Arc<TransactionDB<MultiThreaded>>) -> u32 {
    let meta_cf = db.cf_handle(META_CF).unwrap();
    db.get_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY)
        .unwrap()
        .map_or(0, |height| bytes_to_u32(&height))
}

pub struct EutxoInputIndexer {
    db: Arc<TransactionDB<MultiThreaded>>,
    address_by_hash_lru_cache: LruCache<Vec<u8>, Vec<u8>>,
    hash_by_tx_lru_cache: LruCache<Vec<u8>, Vec<u8>>,
}
impl EutxoInputIndexer {
    pub fn new(db: Arc<TransactionDB<MultiThreaded>>) -> Self {
        Self {
            db,
            address_by_hash_lru_cache: LruCache::new(NonZeroUsize::new(1000_000).unwrap()),
            hash_by_tx_lru_cache: LruCache::new(NonZeroUsize::new(500_000).unwrap()),
        }
    }
}
impl Consumer<Vec<CiBlock>> for EutxoInputIndexer {
    fn consume(&mut self, blocks: &Vec<CiBlock>) {
        let address_cf = self.db.cf_handle(ADDRESS_CF).unwrap();
        let cache_cf = self.db.cf_handle(CACHE_CF).unwrap();
        let meta_cf = self.db.cf_handle(META_CF).unwrap();

        let db_tx = self.db.transaction();
        let mut batch: WriteBatchWithTransaction<true> = db_tx.get_writebatch();

        for block in blocks.iter() {
            for ci_tx in block.txs.iter() {
                process_outputs(
                    ci_tx,
                    &db_tx,
                    &mut batch,
                    &address_cf,
                    &cache_cf,
                    &mut self.address_by_hash_lru_cache,
                    &mut self.hash_by_tx_lru_cache,
                );
                if !ci_tx.is_coinbase {
                    process_inputs(
                        ci_tx,
                        &db_tx,
                        &mut batch,
                        &address_cf,
                        &cache_cf,
                        &mut self.address_by_hash_lru_cache,
                        &mut self.hash_by_tx_lru_cache,
                    );
                }
            }
        }
        // persist last height to db_tx if Some
        blocks.last().map(|block| {
            db_tx
                .put_cf(
                    &meta_cf,
                    LAST_ADDRESS_HEIGHT_KEY,
                    u32_to_bytes(block.height),
                )
                .unwrap();
        });

        db_tx.commit().unwrap();
    }
}
