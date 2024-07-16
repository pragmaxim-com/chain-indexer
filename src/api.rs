use std::cell::RefCell;

use crate::{
    eutxo::eutxo_model::{EuTx, UtxoIndex, UtxoValue},
    indexer::RocksDbBatch,
    model::{BlockHash, BlockHeight, TxCount, TxIndex},
};

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

    fn persist_block(
        &self,
        block: &Self::OutBlock,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String>;

    fn update_blocks(
        &self,
        block: &Vec<Self::OutBlock>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String>;

    fn get_block_height_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<BlockHeight>, rocksdb::Error>;

    fn get_block_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<Self::OutBlock>, rocksdb::Error>;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Vec<EuTx>, String>;

    fn get_utxo_value_by_index(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Vec<(UtxoIndex, UtxoValue)>, String>;

    fn get_block_by_height(
        &self,
        block_height: BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<Self::OutBlock>, rocksdb::Error>;
}

pub trait BlockMonitor<B> {
    fn monitor(&self, block_batch: &Vec<B>, tx_count: &TxCount);
}
