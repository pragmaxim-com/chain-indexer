use lru::LruCache;
use rocksdb::{OptimisticTransactionDB, SingleThreaded, WriteBatchWithTransaction};

use crate::{
    codec_tx::TxPkBytes,
    model::{Block, BlockHash, BlockHeight, Transaction, TxCount, TxHash},
    rocks_db_batch::{CustomFamilies, Families},
};

pub trait BlockchainClient {
    type Tx: Send;

    fn get_best_block(&self) -> Result<Block<Self::Tx>, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::Tx>, String>;
    fn get_block_by_hash(&self, height: BlockHash) -> Result<Block<Self::Tx>, String>;
}

pub trait BlockProcessor {
    type InTx: Send;
    type OutTx: Send;

    fn process(&self, block: &Block<Self::InTx>) -> Block<Self::OutTx>;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount);
}

pub trait BlockProvider {
    type InTx: Send;
    type OutTx: Send;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount);

    fn get_best_block(&self) -> Result<Block<Self::InTx>, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::InTx>, String>;

    fn get_processed_block_by_hash(&self, hash: BlockHash) -> Result<Block<Self::OutTx>, String>;
}
pub trait TxService<'db> {
    type CF: CustomFamilies<'db>;
    type Tx: Transaction;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        families: &Families<'db, Self::CF>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB>,
    ) -> Result<Vec<Self::Tx>, rocksdb::Error>;

    fn persist_txs(
        &self,
        block: &Block<Self::Tx>,
        families: &Families<'db, Self::CF>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        families: &Families<'db, Self::CF>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error>;

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        families: &Families<'db, Self::CF>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error>;
}

pub trait BlockMonitor<Tx> {
    fn monitor(&self, block_batch: &Vec<Block<Tx>>, tx_count: &TxCount);
}

pub struct Storage<'db, CF: CustomFamilies<'db>> {
    pub db: &'db OptimisticTransactionDB<SingleThreaded>,
    pub families: &'db Families<'db, CF>,
}
