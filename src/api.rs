use std::{borrow::Cow, sync::Mutex};

use lru::LruCache;
use rocksdb::{OptimisticTransactionDB, SingleThreaded};

pub type BlockTimestamp = i64;
pub type BlockHeight = u32;
pub type BlockHash = [u8; 32];

pub type TxIndex = u16;
pub type TxHash = [u8; 32];
pub type TxCount = usize;

pub type AssetId = Vec<u8>;
pub type AssetValue = u64;

pub type DbIndexName = Cow<'static, str>;
pub type DbIndexValue = Vec<u8>;

pub trait BlockchainClient {
    type Block: Send;

    fn get_best_block(&self) -> Result<Self::Block, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Self::Block, String>;
    fn get_block_by_hash(&self, height: BlockHash) -> Result<Self::Block, String>;
}

pub trait BlockProcessor {
    type InBlock: Send;
    type OutBlock: Send;

    fn process(&self, block: &Self::InBlock) -> Self::OutBlock;

    fn process_batch(
        &self,
        block_batch: &Vec<Self::InBlock>,
        tx_count: TxCount,
    ) -> (Vec<Self::OutBlock>, TxCount);
}

pub trait ChainLinker {
    type InBlock: Send + Sync;
    type OutBlock: Send + Sync;

    fn process_batch(
        &self,
        block_batch: &Vec<Self::InBlock>,
        tx_count: TxCount,
    ) -> (Vec<Self::OutBlock>, TxCount);

    fn get_best_block(&self) -> Result<Self::InBlock, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Self::InBlock, String>;

    fn get_processed_block_by_hash(&self, hash: BlockHash) -> Result<Self::OutBlock, String>;
}
pub trait Service {
    type OutBlock: Send;

    fn get_tx_pk_by_tx_hash_lru_cache(&self) -> &Mutex<LruCache<[u8; 32], [u8; 6]>>;

    fn process_block(
        &self,
        block: &Self::OutBlock,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        batch: &mut rocksdb::WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<[u8; 32], [u8; 6]>,
    ) -> Result<(), String>;

    fn persist_last_height(
        &self,
        height: BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
    ) -> Result<(), rocksdb::Error>;

    fn get_last_height(&self) -> BlockHeight;

    fn get_block_height_by_hash(
        &self,
        block_hash: &BlockHash,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
    ) -> Result<Option<BlockHeight>, rocksdb::Error>;
}

pub trait BlockMonitor<B> {
    fn monitor(&self, block_batch: &Vec<B>, tx_count: TxCount);
}

pub trait Block {
    fn prev_hash(&self) -> BlockHash;
    fn height(&self) -> BlockHeight;
    fn timestamp(&self) -> BlockTimestamp;
    fn tx_count(&self) -> TxCount;
}
