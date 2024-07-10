use crate::{
    api::{BlockHash, BlockHeight, Service},
    eutxo::eutxo_api::EuTx,
    rocksdb_wrapper::RocksDbWrapper,
};
use lru::LruCache;
use rocksdb::{OptimisticTransactionDB, SingleThreaded};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use super::{eutxo_api::EuBlock, eutxo_codec_block, eutxo_storage};

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

pub struct EuService {
    pub(crate) db: Arc<RocksDbWrapper>,
    pub(crate) tx_pk_by_tx_hash_lru_cache: Mutex<LruCache<[u8; 32], [u8; 6]>>,
}

impl Service for EuService {
    type OutBlock = EuBlock;

    fn get_tx_pk_by_tx_hash_lru_cache(&self) -> &Mutex<LruCache<[u8; 32], [u8; 6]>> {
        &self.tx_pk_by_tx_hash_lru_cache
    }

    fn process_block(
        &self,
        block: &EuBlock,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) -> Result<(), String> {
        self.process_header(&block.height, &block.hash, &db_tx, batch)
            .map_err(|e| e.into_string())?;
        for eu_tx in block.txs.iter() {
            self.process_tx(
                &block.height,
                eu_tx,
                &db_tx,
                batch,
                tx_pk_by_tx_hash_lru_cache,
            )
            .map_err(|e| e.into_string())?;
            self.process_outputs(&block.height, eu_tx, batch);
            if !eu_tx.is_coinbase {
                self.process_inputs(
                    block.height,
                    eu_tx,
                    &db_tx,
                    batch,
                    tx_pk_by_tx_hash_lru_cache,
                );
            }
        }
        Ok(())
    }

    fn persist_last_height(
        &self,
        height: BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.put_cf(
            self.db.borrow_meta_cf(),
            LAST_ADDRESS_HEIGHT_KEY,
            eutxo_codec_block::block_height_to_bytes(&height),
        )
    }

    fn get_last_height(&self) -> BlockHeight {
        self.db
            .borrow_db()
            .get_cf(self.db.borrow_meta_cf(), LAST_ADDRESS_HEIGHT_KEY)
            .unwrap()
            .map_or(0, |height| {
                eutxo_codec_block::vector_to_block_height(&height)
            })
    }

    fn get_block_height_by_hash(
        &self,
        block_hash: &BlockHash,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
    ) -> Result<Option<BlockHeight>, rocksdb::Error> {
        eutxo_storage::get_block_pk_by_hash(block_hash, db_tx, self.db.borrow_block_pk_by_hash_cf())
    }
}

impl EuService {
    pub fn new(db: Arc<RocksDbWrapper>) -> Self {
        EuService {
            db,
            tx_pk_by_tx_hash_lru_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(10_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn process_header(
        &self,
        block_height: &BlockHeight,
        block_hash: &BlockHash,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    ) -> Result<(), rocksdb::Error> {
        eutxo_storage::persist_block_hash_by_pk(
            block_height,
            block_hash,
            batch,
            self.db.borrow_block_hash_by_pk_cf(),
        );
        eutxo_storage::persist_block_pk_by_hash(
            block_hash,
            block_height,
            db_tx,
            self.db.borrow_block_pk_by_hash_cf(),
        )
    }

    pub(crate) fn process_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) -> Result<(), rocksdb::Error> {
        eutxo_storage::persist_tx_hash_by_pk(
            block_height,
            &tx.tx_index,
            &tx.tx_hash,
            batch,
            self.db.borrow_tx_hash_by_pk_cf(),
        );

        eutxo_storage::persist_tx_pk_by_hash(
            block_height,
            &tx.tx_index,
            &tx.tx_hash,
            db_tx,
            self.db.borrow_tx_pk_by_hash_cf(),
            tx_pk_by_tx_hash_lru_cache,
        )
    }

    // Method to process the outputs of a transaction
    pub(crate) fn process_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
    ) {
        for utxo in tx.outs.iter() {
            eutxo_storage::persist_utxo_value_by_pk(
                &block_height,
                &tx.tx_index,
                &utxo.index,
                &utxo.value,
                batch,
                self.db.borrow_utxo_value_by_pk_cf(),
            );

            for (db_index_name, db_index_value) in utxo.db_indexes.iter() {
                eutxo_storage::persist_utxo_index(
                    &db_index_value,
                    &block_height,
                    &tx.tx_index,
                    &utxo.index,
                    batch,
                    &self
                        .db
                        .borrow_index_cf_by_name()
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
        &self,
        block_height: BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
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
                self.db.borrow_utxo_pk_by_input_pk_cf(),
                self.db.borrow_tx_pk_by_hash_cf(),
                tx_pk_by_tx_hash_lru_cache,
            );
        }
    }
}
