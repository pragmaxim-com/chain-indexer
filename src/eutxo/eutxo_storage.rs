use std::sync::Arc;

use lru::LruCache;
use rocksdb::{MultiThreaded, TransactionDB};

use crate::api::{BlockHeight, TxHash, TxIndex};

use super::{
    eutxo_api::{CiTxInput, UtxoIndex, UtxoValue},
    eutxo_tx_codec, eutxo_utxo_codec,
};

pub fn persist_tx_hash_by_pk(
    block_height: BlockHeight,
    tx_index: TxIndex,
    tx_hash: &TxHash,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    tx_hash_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) -> Result<(), rocksdb::Error> {
    let tx_pk_bytes = eutxo_tx_codec::tx_pk_bytes(block_height, tx_index);
    db_tx.put_cf(tx_hash_by_pk_cf, tx_pk_bytes, tx_hash)
}

pub fn persist_tx_pk_by_hash(
    tx_hash: &TxHash,
    block_height: BlockHeight,
    tx_index: TxIndex,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    tx_pk_by_hash_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache: &mut LruCache<Vec<u8>, [u8; 6]>,
) -> Result<(), rocksdb::Error> {
    let tx_pk_bytes: [u8; 6] = eutxo_tx_codec::tx_pk_bytes(block_height, tx_index);
    cache.put(tx_hash.clone(), tx_pk_bytes);
    db_tx.put_cf(tx_pk_by_hash_cf, tx_hash, tx_pk_bytes)
}

pub fn persist_utxo_value_by_pk(
    block_height: BlockHeight,
    tx_index: TxIndex,
    utxo_index: UtxoIndex,
    utxo_value: UtxoValue,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    utxo_value_by_pk_cf: &Arc<rocksdb::BoundColumnFamily>,
) -> Result<(), rocksdb::Error> {
    let utxo_pk_bytes = eutxo_utxo_codec::utxo_pk_bytes(block_height, tx_index, utxo_index);
    let utxo_value_bytes = eutxo_utxo_codec::utxo_value_to_bytes(utxo_value);
    db_tx.put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, utxo_value_bytes)
}

pub fn persist_utxo_pk_by_input_tx(
    tx_input: CiTxInput,
    block_height: BlockHeight,
    tx_index: TxIndex,
    utxo_index: UtxoIndex,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    utxo_pk_by_input_tx_cf: &Arc<rocksdb::BoundColumnFamily>,
    tx_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache: &mut LruCache<Vec<u8>, [u8; 6]>,
) -> Result<(), rocksdb::Error> {
    let utxo_pk_bytes = eutxo_utxo_codec::utxo_pk_bytes(block_height, tx_index, utxo_index);

    let tx_hash_ref = &tx_input.tx_hash;

    let tx_pk_bytes = cache
        .get(tx_hash_ref)
        .map(|f| f.to_vec())
        .or(db_tx.get_cf(tx_cf, tx_hash_ref).unwrap())
        .unwrap();

    let input_pk = eutxo_utxo_codec::utxo_pk_bytes_from(tx_pk_bytes, tx_input.utxo_index);

    db_tx.put_cf(utxo_pk_by_input_tx_cf, input_pk, utxo_pk_bytes)
}
