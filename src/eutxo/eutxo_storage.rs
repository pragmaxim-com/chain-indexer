use std::sync::Arc;

use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB};

use crate::api::{BlockHeight, TxHash, TxIndex};

use super::{
    eutxo_api::{CiTxInput, InputIndex, UtxoIndex, UtxoValue},
    eutxo_tx_codec, eutxo_utxo_codec,
};

pub fn persist_tx_hash_by_pk(
    block_height: BlockHeight,
    tx_index: TxIndex,
    tx_hash: &TxHash,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    tx_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let tx_pk_bytes = eutxo_tx_codec::tx_pk_bytes(block_height, tx_index);
    batch.put_cf(tx_hash_by_pk_cf, tx_pk_bytes, tx_hash)
}

pub fn persist_tx_pk_by_hash(
    block_height: BlockHeight,
    tx_index: TxIndex,
    tx_hash: &TxHash,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
) -> Result<(), rocksdb::Error> {
    let tx_pk_bytes: [u8; 6] = eutxo_tx_codec::tx_pk_bytes(block_height, tx_index);
    tx_pk_by_tx_hash_lru_cache.put(*tx_hash, tx_pk_bytes);
    db_tx.put_cf(tx_pk_by_hash_cf, tx_hash, tx_pk_bytes)
}

pub fn persist_utxo_value_by_pk(
    block_height: BlockHeight,
    tx_index: TxIndex,
    utxo_index: UtxoIndex,
    utxo_value: UtxoValue,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    utxo_value_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    let utxo_pk_bytes = eutxo_utxo_codec::pk_bytes(block_height, tx_index, utxo_index);
    let utxo_value_bytes = eutxo_utxo_codec::utxo_value_to_bytes(utxo_value);
    batch.put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, utxo_value_bytes)
}

pub fn persist_utxo_pk_by_input_pk(
    block_height: BlockHeight,
    tx_index: TxIndex,
    input_index: InputIndex,
    tx_input: &CiTxInput,
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

    let utxo_pk = eutxo_utxo_codec::utxo_pk_bytes_from(tx_pk_bytes, tx_input.utxo_index);
    let input_pk = eutxo_utxo_codec::pk_bytes(block_height, tx_index, input_index);

    batch.put_cf(utxo_pk_by_input_pk_cf, input_pk, utxo_pk)
}
