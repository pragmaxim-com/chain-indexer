use broadcast_sink::Consumer;
use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB, WriteBatchWithTransaction};
use std::{num::NonZeroUsize, sync::Arc};

use crate::{
    api::BlockHeight,
    eutxo::eutxo_api::{CiBlock, CiTx},
};

use super::eutxo_storage;

pub const TX_HASH_BY_PK_CF: &str = "TX_HASH_BY_PK_CF";
pub const TX_PK_BY_HASH_CF: &str = "TX_PK_BY_HASH_CF";
pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
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

fn process_tx(
    block_height: &BlockHeight,
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    tx_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
) -> Result<(), rocksdb::Error> {
    eutxo_storage::persist_tx_hash_by_pk(
        block_height,
        &tx.tx_index,
        &tx.tx_hash,
        batch,
        tx_hash_by_pk_cf,
    );

    eutxo_storage::persist_tx_pk_by_hash(
        block_height,
        &tx.tx_index,
        &tx.tx_hash,
        db_tx,
        tx_pk_by_hash_cf,
        tx_pk_by_tx_hash_lru_cache,
    )
}

// Method to process the outputs of a transaction
fn process_outputs(
    block_height: &BlockHeight,
    tx: &CiTx,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_value_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    for utxo in tx.outs.iter() {
        eutxo_storage::persist_utxo_value_by_pk(
            &block_height,
            &tx.tx_index,
            &utxo.index,
            &utxo.value,
            batch,
            utxo_value_by_pk_cf,
        )
    }
}

// Method to process the inputs of a transaction
fn process_inputs(
    block_height: BlockHeight,
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_pk_by_input_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
) {
    for (input_index, tx_input) in tx.ins.iter().enumerate() {
        eutxo_storage::persist_utxo_pk_by_input_pk(
            &block_height,
            &tx.tx_index,
            &(input_index as u16),
            tx_input,
            db_tx,
            batch,
            utxo_pk_by_input_pk_cf,
            tx_pk_by_hash_cf,
            tx_pk_by_tx_hash_lru_cache,
        );
    }
}

pub fn get_column_families() -> Vec<&'static str> {
    vec![
        META_CF,
        TX_HASH_BY_PK_CF,
        TX_PK_BY_HASH_CF,
        UTXO_VALUE_BY_PK_CF,
        UTXO_PK_BY_INPUT_PK_CF,
    ]
}

pub fn get_last_height(db: Arc<TransactionDB<MultiThreaded>>) -> u32 {
    let meta_cf = db.cf_handle(META_CF).unwrap();
    db.get_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY)
        .unwrap()
        .map_or(0, |height| bytes_to_u32(&height))
}

pub struct EutxoInputIndexer {
    db: Arc<TransactionDB<MultiThreaded>>,
    tx_pk_by_tx_hash_lru_cache: LruCache<[u8; 32], [u8; 6]>,
}
impl EutxoInputIndexer {
    pub fn new(db: Arc<TransactionDB<MultiThreaded>>) -> Self {
        Self {
            db,
            tx_pk_by_tx_hash_lru_cache: LruCache::new(NonZeroUsize::new(10_000_000).unwrap()),
        }
    }
}
impl Consumer<Vec<CiBlock>> for EutxoInputIndexer {
    fn consume(&mut self, blocks: &Vec<CiBlock>) {
        let tx_hash_by_pk_cf = self.db.cf_handle(TX_HASH_BY_PK_CF).unwrap();
        let tx_pk_by_hash_cf = self.db.cf_handle(TX_PK_BY_HASH_CF).unwrap();
        let utxo_value_by_pk_cf = self.db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap();
        let utxo_pk_by_input_pk_cf = self.db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap();
        let meta_cf = self.db.cf_handle(META_CF).unwrap();

        let db_tx = self.db.transaction();
        let mut batch: WriteBatchWithTransaction<true> = db_tx.get_writebatch();

        for block in blocks.iter() {
            for ci_tx in block.txs.iter() {
                process_tx(
                    &block.height,
                    ci_tx,
                    &db_tx,
                    &mut batch,
                    &tx_hash_by_pk_cf,
                    &tx_pk_by_hash_cf,
                    &mut self.tx_pk_by_tx_hash_lru_cache,
                )
                .unwrap();
                process_outputs(&block.height, ci_tx, &mut batch, &utxo_value_by_pk_cf);
                if !ci_tx.is_coinbase {
                    process_inputs(
                        block.height,
                        ci_tx,
                        &db_tx,
                        &mut batch,
                        &utxo_pk_by_input_pk_cf,
                        &tx_pk_by_hash_cf,
                        &mut self.tx_pk_by_tx_hash_lru_cache,
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
