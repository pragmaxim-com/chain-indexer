use std::{pin::Pin, sync::Arc};

use crate::{
    codec_tx::TxPkBytes,
    eutxo::eutxo_index_manager::DbIndexManager,
    model::{Block, BlockHeader, BlockHeight, Transaction, TxCount, TxHash},
    rocks_db_batch::{CustomFamilies, Families},
};
use async_trait::async_trait;
use futures::Stream;
use lru::LruCache;
use rocksdb::{MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction};

pub trait BlockProcessor {
    type InTx: Send;
    type OutTx: Send;

    fn process_block(&self, block: &Block<Self::InTx>) -> Block<Self::OutTx>;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount);
}

#[async_trait]
pub trait BlockProvider {
    type OutTx: Send;

    fn get_index_manager(&self) -> DbIndexManager;

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block<Self::OutTx>, String>;

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<Self::OutTx>>, TxCount)> + Send + 'life0>>;
}

pub trait TxService<'db> {
    type CF: CustomFamilies<'db>;
    type Tx: Transaction;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, Self::CF>,
    ) -> Result<Vec<Self::Tx>, rocksdb::Error>;

    fn persist_txs(
        &self,
        block: &Block<Self::Tx>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;
}

pub trait BlockMonitor<Tx> {
    fn monitor(&self, block_batch: &Vec<Block<Tx>>, tx_count: &TxCount);
}

pub struct Storage {
    pub db: Arc<OptimisticTransactionDB<MultiThreaded>>,
}
