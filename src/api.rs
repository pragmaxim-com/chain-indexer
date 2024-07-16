use std::cell::RefCell;

use crate::{
    eutxo::eutxo_model::{EuTx, UtxoIndex, UtxoValue},
    indexer::RocksDbBatch,
    model::{Block, BlockHash, BlockHeight, TxCount, TxIndex},
};

pub trait BlockchainClient {
    type Tx: Send + Clone;

    fn get_best_block(&self) -> Result<Block<Self::Tx>, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::Tx>, String>;
    fn get_block_by_hash(&self, height: BlockHash) -> Result<Block<Self::Tx>, String>;
}

pub trait BlockProcessor {
    type InTx: Send + Clone;
    type OutTx: Send + Clone;

    fn process(&self, block: &Block<Self::InTx>) -> Block<Self::OutTx>;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount);
}

pub trait ChainLinker {
    type InTx: Send + Clone;
    type OutTx: Send + Clone;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount);

    fn get_best_block(&self) -> Result<Block<Self::InTx>, String>;

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::InTx>, String>;

    fn get_processed_block_by_hash(&self, hash: BlockHash) -> Result<Block<Self::OutTx>, String>;
}
pub trait Service {
    type OutTx: Clone;

    fn persist_block(
        &self,
        block: Block<Self::OutTx>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String>;

    fn update_blocks(
        &self,
        block: &Vec<Block<Self::OutTx>>,
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
    ) -> Result<Option<Block<Self::OutTx>>, rocksdb::Error>;

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
    ) -> Result<Option<Block<Self::OutTx>>, rocksdb::Error>;
}

pub trait BlockMonitor<Tx: Clone> {
    fn monitor(&self, block_batch: &Vec<Block<Tx>>, tx_count: &TxCount);
}
