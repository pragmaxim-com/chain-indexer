use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB};
use std::sync::Arc;

use crate::{
    api::{BlockHash, BlockHeight, DbIndexName},
    eutxo::eutxo_api::EuTx,
};

use super::eutxo_storage;

pub(crate) fn process_block(
    block_height: &BlockHeight,
    block_hash: &BlockHash,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    block_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    block_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
) -> Result<(), rocksdb::Error> {
    eutxo_storage::persist_block_hash_by_pk(block_height, block_hash, batch, block_hash_by_pk_cf);
    eutxo_storage::persist_block_pk_by_hash(block_hash, block_height, db_tx, block_pk_by_hash_cf)
}

pub(crate) fn process_tx(
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
pub(crate) fn process_outputs(
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
pub(crate) fn process_inputs(
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
