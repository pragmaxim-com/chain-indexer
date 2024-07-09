use std::sync::Arc;

use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB};

use crate::api::{BlockHash, BlockHeight, DbIndexValue, TxHash, TxIndex};

use super::{
    eutxo_api::{EuTxInput, InputIndex, UtxoIndex, UtxoValue},
    eutxo_codec_block, eutxo_codec_tx, eutxo_codec_utxo,
};

pub fn persist_block_hash_by_pk(
    block_height: &BlockHeight,
    block_hash: &BlockHash,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    block_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let height_bytes = eutxo_codec_block::block_height_to_bytes(block_height);
    batch.put_cf(block_hash_by_pk_cf, height_bytes, block_hash)
}

pub fn get_block_pk_by_hash(
    block_hash: &BlockHash,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    block_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
) -> Result<Option<BlockHeight>, rocksdb::Error> {
    let height_bytes = db_tx.get_cf(block_pk_by_hash_cf, block_hash)?;
    Ok(height_bytes.map(|bytes| eutxo_codec_block::vector_to_block_height(&bytes)))
}

pub fn persist_block_pk_by_hash(
    block_hash: &BlockHash,
    block_height: &BlockHeight,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    block_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
) -> Result<(), rocksdb::Error> {
    let height_bytes = eutxo_codec_block::block_height_to_bytes(block_height);
    db_tx.put_cf(block_pk_by_hash_cf, block_hash, height_bytes)
}

pub fn persist_tx_hash_by_pk(
    block_height: &BlockHeight,
    tx_index: &TxIndex,
    tx_hash: &TxHash,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    tx_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let tx_pk_bytes = eutxo_codec_tx::tx_pk_bytes(block_height, tx_index);
    batch.put_cf(tx_hash_by_pk_cf, tx_pk_bytes, tx_hash)
}

pub fn persist_tx_pk_by_hash(
    block_height: &BlockHeight,
    tx_index: &TxIndex,
    tx_hash: &TxHash,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
) -> Result<(), rocksdb::Error> {
    let tx_pk_bytes: [u8; 6] = eutxo_codec_tx::tx_pk_bytes(block_height, tx_index);
    tx_pk_by_tx_hash_lru_cache.put(*tx_hash, tx_pk_bytes);
    db_tx.put_cf(tx_pk_by_hash_cf, tx_hash, tx_pk_bytes)
}

pub fn persist_utxo_value_by_pk(
    block_height: &BlockHeight,
    tx_index: &TxIndex,
    utxo_index: &UtxoIndex,
    utxo_value: &UtxoValue,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_value_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let utxo_pk_bytes = eutxo_codec_utxo::pk_bytes(block_height, tx_index, utxo_index);
    let utxo_value_bytes = eutxo_codec_utxo::utxo_value_to_bytes(utxo_value);
    batch.put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, utxo_value_bytes)
}

pub fn persist_utxo_pk_by_input_pk(
    block_height: &BlockHeight,
    tx_index: &TxIndex,
    input_index: &InputIndex,
    tx_input: &EuTxInput,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_pk_by_input_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
) {
    let tx_pk_bytes = tx_pk_by_tx_hash_lru_cache
        .get(&tx_input.tx_hash)
        .map(|f| f.to_vec())
        .or(db_tx.get_cf(tx_pk_by_hash_cf, tx_input.tx_hash).unwrap())
        .unwrap();

    let utxo_pk = eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, tx_input.utxo_index);
    let input_pk = eutxo_codec_utxo::pk_bytes(block_height, tx_index, input_index);

    batch.put_cf(utxo_pk_by_input_pk_cf, input_pk, utxo_pk)
}

pub fn persist_utxo_index(
    db_index_value: &DbIndexValue,
    block_height: &BlockHeight,
    tx_index: &TxIndex,
    utxo_index: &UtxoIndex,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_index_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let utxo_pk = eutxo_codec_utxo::pk_bytes(block_height, tx_index, utxo_index);

    batch.merge_cf(utxo_index_cf, db_index_value, utxo_pk)
}
