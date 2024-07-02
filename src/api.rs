use std::{borrow::Cow, sync::Arc};

use broadcast_sink::Consumer;
use tokio::sync::Mutex;

pub type BlockTimestamp = i64;
pub type BlockHeight = u32;
pub type BlockHash = Vec<u8>;

pub type TxIndex = u16;
pub type TxHash = [u8; 32];

pub type AssetId = Vec<u8>;
pub type AssetValue = u64;

pub type DbIndexName = Cow<'static, str>;
pub type DbIndexValue = Vec<u8>;

pub trait BlockchainClient {
    type Block: Send;

    fn get_block_with_tx_count_for_height(
        &self,
        height: u32,
    ) -> Result<(Self::Block, usize), String>;
}

pub trait BlockProcessor {
    type InBlock: Send;
    type OutBlock: Send;
    fn process(
        &self,
        block_batch: &Vec<(BlockHeight, Self::InBlock, usize)>,
    ) -> Vec<Self::OutBlock>;
}

pub trait Indexers {
    type OutBlock: Send;
    fn get_last_height(&self) -> u32;
    fn get_indexers(&self) -> Vec<Arc<Mutex<dyn Consumer<Vec<Self::OutBlock>>>>>;
}

pub trait Syncable {
    fn sync(&self, start_height: BlockHeight, end_height: BlockHeight) -> Result<(), String>;
}

pub struct ChainSyncer<InBlock: Send, OutBlock: Send> {
    pub client: Arc<dyn BlockchainClient<Block = InBlock> + Send + Sync>,
    pub processor: Arc<dyn BlockProcessor<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    pub indexers: Arc<dyn Indexers<OutBlock = OutBlock>>,
}
