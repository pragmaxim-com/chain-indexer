use std::sync::Arc;

use tokio::task::JoinHandle;

pub type Height = u64;
pub type Address = String;
pub type ScriptHash = [u8; 32];
pub type TxId = [u8; 32];
pub type Amount = u64;
pub type UtxoIndex = usize;
pub type BlockHash = String;
pub type Time = i64;

#[derive(Debug, Clone)]
pub struct CiUtxo {
    pub index: UtxoIndex,
    pub address: Option<Address>,
    pub script_hash: ScriptHash,
    pub value: Amount,
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
    pub ins: Vec<CiIndexedTxid>,
    pub outs: Vec<CiUtxo>,
}

#[derive(Debug, Clone)]
pub struct CiBlock {
    pub hash: BlockHash,
    pub height: Height,
    pub time: Time,
    pub txs: Vec<CiTx>,
}

pub trait BlockchainClient {
    type Block: Send;
    type BlockHash;

    fn get_block_with_tx_count_for_height(
        &self,
        height: u64,
    ) -> Result<(Self::Block, usize), String>;

    fn get_block_hash(&self, height: u64) -> Result<Self::BlockHash, String>;

    fn get_block_with_tx_count(
        &self,
        hash: &Self::BlockHash,
    ) -> Result<(Self::Block, usize), String>;
}

pub trait BlockProcessor {
    type Block: Send;
    fn process(&self, block_batch: Vec<(Height, Self::Block, usize)>) -> Vec<(Height, CiBlock)>;
}

pub trait BlockBatchIndexer: Send + Sync {
    fn get_last_height(&self) -> u64;
    fn index(&self, block_batch: Arc<Vec<(Height, CiBlock)>>) -> Vec<JoinHandle<()>>;
}

pub trait Syncable {
    fn sync(&self, start_height: Height, end_height: Height) -> Result<(), String>;
}

pub struct ChainSyncer<B: Send, BH> {
    pub client: Arc<dyn BlockchainClient<Block = B, BlockHash = BH> + Send + Sync>,
    pub processor: Arc<dyn BlockProcessor<Block = B> + Send + Sync>,
    pub indexer: Arc<dyn BlockBatchIndexer>,
}