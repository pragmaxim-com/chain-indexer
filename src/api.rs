use std::{borrow::Cow, sync::Arc};

use broadcast_sink::Consumer;
use tokio::sync::Mutex;

pub type BlockHeight = u32;
pub type BlockHash = Vec<u8>;
pub type TxIndex = u16;
pub type TxId = Vec<u8>;
pub type Value = u64;
pub type UtxoIndex = u16;
pub type Time = i64;
pub type TokenId = Vec<u8>;
pub type TokenValue = u64;
pub type IndexName = Cow<'static, str>;
pub type IndexValue = Vec<u8>;

// define constant for address and script_hash
pub const ADDRESS_INDEX: &str = "address";
pub const SCRIPT_HASH_INDEX: &str = "script_hash";

#[derive(Debug, Clone)]
pub struct CiUtxo {
    pub index: UtxoIndex,
    pub db_indexes: Vec<(IndexName, IndexValue)>,
    pub assets: Vec<(TokenId, TokenValue)>,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct CiIndexedTxid {
    pub tx_id: TxId,
    pub utxo_index: UtxoIndex,
}

#[derive(Debug, Clone)]
pub struct CiTx {
    pub is_coinbase: bool,
    pub tx_id: TxId,
    pub tx_index: TxIndex,
    pub ins: Vec<CiIndexedTxid>,
    pub outs: Vec<CiUtxo>,
}

#[derive(Debug, Clone)]
pub struct CiBlock {
    pub hash: BlockHash,
    pub height: BlockHeight,
    pub time: Time,
    pub txs: Vec<CiTx>,
}

pub trait BlockchainClient {
    type Block: Send;
    type BlockHash;

    fn get_block_with_tx_count_for_height(
        &self,
        height: u32,
    ) -> Result<(Self::Block, usize), String>;

    fn get_block_hash(&self, height: u64) -> Result<Self::BlockHash, String>;

    fn get_block_with_tx_count(
        &self,
        hash: &Self::BlockHash,
    ) -> Result<(Self::Block, usize), String>;
}

pub trait BlockProcessor {
    type Block: Send;
    fn process(&self, block_batch: Vec<(BlockHeight, Self::Block, usize)>) -> Vec<CiBlock>;
}

pub trait Storage: Send + Sync {
    fn get_last_height(&self) -> u32;
    fn get_indexers(&self) -> Vec<Arc<Mutex<dyn Consumer<Vec<CiBlock>>>>>;
}

pub trait Syncable {
    fn sync(&self, start_height: BlockHeight, end_height: BlockHeight) -> Result<(), String>;
}

pub struct ChainSyncer<B: Send, BH> {
    pub client: Arc<dyn BlockchainClient<Block = B, BlockHash = BH> + Send + Sync>,
    pub processor: Arc<dyn BlockProcessor<Block = B> + Send + Sync>,
    pub storage: Arc<dyn Storage>,
}
