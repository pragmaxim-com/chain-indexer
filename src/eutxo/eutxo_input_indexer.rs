use broadcast_sink::{BroadcastSinkError, Consumer};
use lru::LruCache;
use rocksdb::{BoundColumnFamily, MultiThreaded, TransactionDB, WriteBatchWithTransaction};
use std::{num::NonZeroUsize, sync::Arc};

use crate::{
    api::{BlockHash, BlockHeight, DbIndexName},
    eutxo::eutxo_api::{EuBlock, EuTx},
};

use super::eutxo_storage;

pub const BLOCK_HASH_BY_PK_CF: &str = "BLOCK_HASH_BY_PK_CF";
pub const BLOCK_PK_BY_HASH_CF: &str = "BLOCK_PK_BY_HASH_CF";
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

fn process_block(
    block_height: &BlockHeight,
    block_hash: &BlockHash,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    block_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    block_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    eutxo_storage::persist_block_hash_by_pk(block_height, block_hash, batch, block_hash_by_pk_cf);
    eutxo_storage::persist_block_pk_by_hash(block_hash, block_height, batch, block_pk_by_hash_cf);
}

fn process_tx(
    block_height: &BlockHeight,
    tx: &EuTx,
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
    tx: &EuTx,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_value_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    utxo_indexes: &Vec<(DbIndexName, Arc<rocksdb::BoundColumnFamily>)>,
) {
    for utxo in tx.outs.iter() {
        eutxo_storage::persist_utxo_value_by_pk(
            &block_height,
            &tx.tx_index,
            &utxo.index,
            &utxo.value,
            batch,
            &utxo_value_by_pk_cf,
        );

        for (db_index_name, db_index_value) in utxo.db_indexes.iter() {
            eutxo_storage::persist_utxo_index(
                &db_index_value,
                &block_height,
                &tx.tx_index,
                &utxo.index,
                batch,
                &utxo_indexes
                    .iter()
                    .find(|&i| db_index_name == &i.0)
                    .unwrap()
                    .1,
            )
        }
    }
}

// Method to process the inputs of a transaction
fn process_inputs(
    block_height: BlockHeight,
    tx: &EuTx,
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
        BLOCK_HASH_BY_PK_CF,
        BLOCK_PK_BY_HASH_CF,
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
    utxo_indexes: Vec<DbIndexName>,
}
impl EutxoInputIndexer {
    pub fn new(db: Arc<TransactionDB<MultiThreaded>>, utxo_indexes: Vec<DbIndexName>) -> Self {
        Self {
            db,
            tx_pk_by_tx_hash_lru_cache: LruCache::new(NonZeroUsize::new(10_000_000).unwrap()),
            utxo_indexes,
        }
    }
}
impl Consumer<Vec<EuBlock>> for EutxoInputIndexer {
    fn consume(&mut self, blocks: &Vec<EuBlock>) -> Result<(), BroadcastSinkError> {
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

        for block in blocks.iter() {
            process_block(
                &block.height,
                &block.hash,
                &mut batch,
                &block_hash_by_pk_cf,
                &block_pk_by_hash_cf,
            );
            for eu_tx in block.txs.iter() {
                process_tx(
                    &block.height,
                    eu_tx,
                    &db_tx,
                    &mut batch,
                    &tx_hash_by_pk_cf,
                    &tx_pk_by_hash_cf,
                    &mut self.tx_pk_by_tx_hash_lru_cache,
                )
                .map_err(|e| BroadcastSinkError::new(e.as_ref()))?;
                process_outputs(
                    &block.height,
                    eu_tx,
                    &mut batch,
                    &utxo_value_by_pk_cf,
                    index_cf_by_name,
                );
                if !eu_tx.is_coinbase {
                    process_inputs(
                        block.height,
                        eu_tx,
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
        if let Some(block) = blocks.last() {
            db_tx
                .put_cf(
                    &meta_cf,
                    LAST_ADDRESS_HEIGHT_KEY,
                    u32_to_bytes(block.height),
                )
                .map_err(|e| BroadcastSinkError::new(e.as_ref()))?;
        }
        db_tx
            .commit()
            .map_err(|e| BroadcastSinkError::new(e.as_ref()))?;
        Ok(())
    }
}
