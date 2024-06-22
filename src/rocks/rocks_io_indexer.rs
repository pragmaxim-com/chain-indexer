use super::rocks_storage::{ADDRESS_CF, CACHE_CF, LAST_ADDRESS_HEIGHT_KEY, META_CF};
use crate::api::{CiBlock, CiTx, Height};
use broadcast_sink::Consumer;
use rocksdb::{MultiThreaded, TransactionDB, WriteBatchWithTransaction};
use std::sync::Arc;

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
        let mut tx_id_with_index = Vec::with_capacity(tx.tx_id.len() + 2);
        tx_id_with_index.extend_from_slice(&tx.tx_id);
        tx_id_with_index.push(b'|');
        tx_id_with_index.push(utxo.index);

        let mut utxo_hash_with_value = Vec::with_capacity(utxo.script_hash.len() + 9);
        utxo_hash_with_value.extend_from_slice(&utxo.script_hash);
        utxo_hash_with_value.push(b'|');
        utxo_hash_with_value.extend_from_slice(&u64_to_bytes(utxo.value));

        db_tx
            .put_cf(cache_cf, &tx_id_with_index, &utxo_hash_with_value)
            .unwrap();

        let mut address_key = Vec::with_capacity(utxo.script_hash.len() + tx.tx_id.len() + 14);
        address_key.extend_from_slice(&utxo.script_hash);
        address_key.push(b'|');
        address_key.extend_from_slice(utxo.address.as_ref().unwrap_or(&Vec::new()));
        address_key.push(b'|');
        address_key.push(b'O');
        address_key.push(b'|');
        address_key.extend_from_slice(&tx.tx_id);
        address_key.push(b'|');
        address_key.push(utxo.index);
        address_key.push(b'|');
        address_key.extend_from_slice(&u64_to_bytes(utxo.value));

        batch.put_cf(address_cf, &address_key, &[]);
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
        let mut tx_cache_key = Vec::with_capacity(indexed_txid.tx_id.len() + 2);
        tx_cache_key.extend_from_slice(&indexed_txid.tx_id);
        tx_cache_key.push(b'|'); // Adding the delimiter '|'
        tx_cache_key.push(indexed_txid.utxo_index);

        let utxo_str = db_tx.get_cf(cache_cf, &tx_cache_key).unwrap().unwrap();
        let splits: Vec<&[u8]> = utxo_str.split(|&byte| byte == b'|').collect();
        let script_hash = splits[0];
        let value = splits[1];
        let mut iter = db_tx.prefix_iterator_cf(address_cf, &script_hash);

        let first_address = iter.next().and_then(|result| {
            match result {
                Ok((key, _)) => {
                    let parts: Vec<&[u8]> = key.split(|&byte| byte == b'|').collect();
                    if parts.len() > 1 {
                        Some(parts[1].to_vec())
                    } else {
                        Some(vec![]) // Return an empty vector if the address is empty
                    }
                }
                Err(_) => None,
            }
        });
        let mut address_key = Vec::with_capacity(script_hash.len() + indexed_txid.tx_id.len() + 14);
        address_key.extend_from_slice(script_hash);
        address_key.push(b'|');
        address_key.extend_from_slice(first_address.as_ref().unwrap_or(&Vec::new()));
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.push(b'I');
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.extend_from_slice(&indexed_txid.tx_id);
        address_key.push(b'|'); // Adding the delimiter '|'
        address_key.push(indexed_txid.utxo_index);
        address_key.push(b'|');
        address_key.extend_from_slice(value);

        batch.put_cf(address_cf, address_key, &[]);
    }
}

pub fn get_last_height(db: Arc<TransactionDB<MultiThreaded>>) -> u64 {
    let meta_cf = db.cf_handle(META_CF).unwrap();
    db.get_cf(&meta_cf, LAST_ADDRESS_HEIGHT_KEY)
        .unwrap()
        .map_or(0, |height| bytes_to_u64(&height))
}

pub struct RocksIoIndexer {
    db: Arc<TransactionDB<MultiThreaded>>,
}
impl RocksIoIndexer {
    pub fn new(db: Arc<TransactionDB<MultiThreaded>>) -> Self {
        Self { db }
    }
}
impl Consumer<Vec<(Height, CiBlock)>> for RocksIoIndexer {
    fn consume(&self, blocks: &Vec<(Height, CiBlock)>) {
        let address_cf = self.db.cf_handle(ADDRESS_CF).unwrap();
        let cache_cf = self.db.cf_handle(CACHE_CF).unwrap();
        let meta_cf = self.db.cf_handle(META_CF).unwrap();

        let db_tx = self.db.transaction();
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
}
