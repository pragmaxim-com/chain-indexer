use std::cell::{RefCell, RefMut};

use lru::LruCache;

use crate::{
    model::{Block, BlockHash, BlockHeight, Transaction, TxCount, TxHash},
    rocks_db_batch::RocksDbBatch,
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
pub trait TxService {
    type Tx: Transaction + Clone;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Vec<Self::Tx>, rocksdb::Error>;

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, [u8; 6]>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        batch: &mut RefMut<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, [u8; 6]>,
    );
}

pub trait BlockMonitor<Tx: Clone> {
    fn monitor(&self, block_batch: &Vec<Block<Tx>>, tx_count: &TxCount);
}
