use crate::api::{CiBlock, CiTx, Height};
use rocksdb::{MultiThreaded, TransactionDB, WriteBatchWithTransaction};
use std::sync::Arc;

use super::rocks_indexer::{ADDRESS_CF, CACHE_CF, LAST_ADDRESS_HEIGHT_KEY, META_CF};

fn usize_to_bytes(n: usize) -> [u8; std::mem::size_of::<usize>()] {
    n.to_ne_bytes()
}
fn u64_to_bytes(n: u64) -> [u8; std::mem::size_of::<u64>()] {
    n.to_ne_bytes()
}

fn bytes_to_u64(bytes: &[u8]) -> u64 {
    let mut array = [0u8; std::mem::size_of::<u64>()];
    array.copy_from_slice(bytes);
    u64::from_ne_bytes(array)
}

// Method to process the outputs of a transaction
fn process_outputs(
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    address_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    for utxo in tx.outs.iter() {
        let mut tx_id_with_index = Vec::new();
        tx_id_with_index.extend_from_slice(&tx.tx_id);
        tx_id_with_index.push(b'|');
        tx_id_with_index.extend_from_slice(&usize_to_bytes(utxo.index));

        let mut utxo_hash_with_value = Vec::new();
        utxo_hash_with_value.extend_from_slice(&utxo.script_hash);
        utxo_hash_with_value.push(b'|');
        utxo_hash_with_value.extend_from_slice(&u64_to_bytes(utxo.value));

        let mut address_key = Vec::new();
        address_key.extend_from_slice(&utxo.script_hash);
        address_key.push(b'|');
        address_key.push(b'O');
        address_key.push(b'|');
        address_key.extend_from_slice(&tx.tx_id);
        address_key.push(b'|');
        address_key.extend_from_slice(&usize_to_bytes(utxo.index));

        db_tx
            .put_cf(cache_cf, &tx_id_with_index, &utxo_hash_with_value)
            .unwrap();
        batch.put_cf(address_cf, &address_key, &u64_to_bytes(utxo.value));
    }
}

// Method to process the inputs of a transaction
fn process_inputs(
    tx: &CiTx,
    db_tx: &rocksdb::Transaction<TransactionDB<MultiThreaded>>,
    batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    address_cf: &Arc<rocksdb::BoundColumnFamily>,
    cache_cf: &Arc<rocksdb::BoundColumnFamily>,
) {
    for indexed_txid in &tx.ins {
        let mut tx_cache_key = Vec::new();
        tx_cache_key.extend_from_slice(&indexed_txid.tx_id);
        tx_cache_key.push(b'|'); // Adding the delimiter '|'
        tx_cache_key.extend_from_slice(&usize_to_bytes(indexed_txid.utxo_index));

        let utxo_str = db_tx.get_cf(cache_cf, &tx_cache_key).unwrap().unwrap();
        let splits: Vec<&[u8]> = utxo_str.split(|&byte| byte == b'|').collect();
        let hash = splits[0];
        let value = splits[1];

        let mut address_key = Vec::new();
        address_key.extend_from_slice(hash);
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.push(b'I');
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.extend_from_slice(&indexed_txid.tx_id);
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.extend_from_slice(&usize_to_bytes(indexed_txid.utxo_index));

        batch.put_cf(address_cf, address_key, value);
    }
}

pub fn get_last_height(db: Arc<TransactionDB<MultiThreaded>>) -> u64 {
    let meta_cf = db.cf_handle(META_CF).unwrap();
    db.get_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY)
        .unwrap()
        .map_or(0, |height| bytes_to_u64(&height))
}

pub fn index_blocks(db: Arc<TransactionDB<MultiThreaded>>, blocks: &Vec<(Height, CiBlock)>) {
    let address_cf = db.cf_handle(ADDRESS_CF).unwrap();
    let cache_cf = db.cf_handle(CACHE_CF).unwrap();
    let meta_cf = db.cf_handle(META_CF).unwrap();

    let db_tx = db.transaction();
    let mut batch: WriteBatchWithTransaction<true> = db_tx.get_writebatch();

    for (_, block) in blocks.iter() {
        for sum_tx in block.txs.iter() {
            process_outputs(sum_tx, &db_tx, &mut batch, &address_cf, &cache_cf);
            if !sum_tx.is_coinbase {
                process_inputs(sum_tx, &db_tx, &mut batch, &address_cf, &cache_cf);
            }
        }
    }
    // let get last height
    let last_height = blocks.iter().last().unwrap().0;
    db_tx
        .put_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY, u64_to_bytes(last_height))
        .unwrap();

    db_tx.commit().unwrap();
}
