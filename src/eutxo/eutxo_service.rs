use crate::{
    api::{BlockHash, BlockHeight, Service},
    eutxo::eutxo_api::EuTx,
    indexer::RocksDbBatch,
};
use lru::LruCache;
use std::{
    cell::{RefCell, RefMut},
    num::NonZeroUsize,
    sync::Mutex,
};

use super::{eutxo_api::EuBlock, eutxo_codec_block, eutxo_codec_tx, eutxo_codec_utxo};

pub struct EuService {
    pub(crate) tx_pk_by_tx_hash_lru_cache: Mutex<LruCache<[u8; 32], [u8; 6]>>,
}

impl<'d> Service for EuService {
    type OutBlock = EuBlock;

    fn get_tx_pk_by_tx_hash_lru_cache(&self) -> &Mutex<LruCache<[u8; 32], [u8; 6]>> {
        &self.tx_pk_by_tx_hash_lru_cache
    }

    fn persist_block(
        &self,
        block: &EuBlock,
        batch: &RefCell<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) -> Result<(), String> {
        let mut batch = batch.borrow_mut();
        self.persist_header(&block.height, &block.hash, &mut batch)
            .map_err(|e| e.into_string())?;
        for eu_tx in block.txs.iter() {
            self.persist_tx(&block.height, eu_tx, &mut batch, tx_pk_by_tx_hash_lru_cache)
                .map_err(|e| e.into_string())?;
            self.persist_outputs(&block.height, eu_tx, &mut batch);
            if !eu_tx.is_coinbase {
                self.persist_inputs(&block.height, eu_tx, &mut batch, tx_pk_by_tx_hash_lru_cache);
            }
        }
        Ok(())
    }

    fn get_block_height_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<BlockHeight>, rocksdb::Error> {
        let batch = batch.borrow_mut();
        let height_bytes = batch.db_tx.get_cf(batch.block_pk_by_hash_cf, block_hash)?;
        Ok(height_bytes.map(|bytes| eutxo_codec_block::vector_to_block_height(&bytes)))
    }
}

impl EuService {
    pub fn new() -> Self {
        EuService {
            tx_pk_by_tx_hash_lru_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(10_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn persist_header(
        &self,
        block_height: &BlockHeight,
        block_hash: &BlockHash,
        batch: &mut RefMut<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error> {
        let height_bytes = eutxo_codec_block::block_height_to_bytes(block_height);
        let block_hash_by_pk_cf = batch.block_hash_by_pk_cf;
        batch
            .batch
            .put_cf(&block_hash_by_pk_cf, height_bytes, block_hash);

        let height_bytes = eutxo_codec_block::block_height_to_bytes(block_height);
        batch
            .db_tx
            .put_cf(batch.block_pk_by_hash_cf, block_hash, height_bytes)
    }

    pub(crate) fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = eutxo_codec_tx::tx_pk_bytes(block_height, &tx.tx_index);
        let tx_hash_by_pk_cf = batch.tx_hash_by_pk_cf;
        let tx_pk_by_hash_cf = batch.tx_pk_by_hash_cf;
        batch
            .batch
            .put_cf(tx_hash_by_pk_cf, tx_pk_bytes, &tx.tx_hash);

        let tx_pk_bytes: [u8; 6] = eutxo_codec_tx::tx_pk_bytes(block_height, &tx.tx_index);
        tx_pk_by_tx_hash_lru_cache.put(tx.tx_hash, tx_pk_bytes);
        batch
            .db_tx
            .put_cf(tx_pk_by_hash_cf, &tx.tx_hash, tx_pk_bytes)
    }

    // Method to process the outputs of a transaction
    pub(crate) fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
    ) {
        for utxo in tx.outs.iter() {
            let utxo_pk_bytes = eutxo_codec_utxo::pk_bytes(block_height, &tx.tx_index, &utxo.index);
            let utxo_value_bytes = eutxo_codec_utxo::utxo_value_to_bytes(&utxo.value);
            let utxo_value_by_pk_cf = batch.utxo_value_by_pk_cf;
            batch
                .batch
                .put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, utxo_value_bytes);

            for (db_index_name, db_index_value) in utxo.db_indexes.iter() {
                let utxo_pk = eutxo_codec_utxo::pk_bytes(block_height, &tx.tx_index, &utxo.index);
                let utxo_index_cf = batch
                    .index_cf_by_name
                    .iter()
                    .find(|&i| db_index_name == &i.0)
                    .unwrap()
                    .1;
                batch.batch.merge_cf(utxo_index_cf, db_index_value, utxo_pk)
            }
        }
    }

    // Method to process the inputs of a transaction
    pub(crate) fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) {
        for (input_index, tx_input) in tx.ins.iter().enumerate() {
            let tx_pk_bytes = tx_pk_by_tx_hash_lru_cache
                .get(&tx_input.tx_hash)
                .map(|f| f.to_vec())
                .or(batch
                    .db_tx
                    .get_cf(batch.tx_pk_by_hash_cf, tx_input.tx_hash)
                    .unwrap())
                .unwrap();

            let utxo_pk = eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, tx_input.utxo_index);
            let input_pk =
                eutxo_codec_utxo::pk_bytes(block_height, &tx.tx_index, &(input_index as u16));
            let utxo_pk_by_input_pk_cf = batch.utxo_pk_by_input_pk_cf;
            batch
                .batch
                .put_cf(utxo_pk_by_input_pk_cf, input_pk, utxo_pk)
        }
    }
}
