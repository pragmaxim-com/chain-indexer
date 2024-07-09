use std::borrow::Cow;

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

pub trait Indexer {
    type OutBlock: Send;
    fn get_last_height(&self) -> u32;
    fn consume(&self, blocks: &Vec<Self::OutBlock>) -> Result<(), String>;
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
