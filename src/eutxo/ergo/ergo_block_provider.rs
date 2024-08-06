use ergo_lib::chain::transaction::Transaction;
use reqwest::Url;

use crate::{
    api::{BlockProcessor, BlockProvider},
    eutxo::eutxo_model::EuTx,
    model::{Block, BlockHash, BlockHeight, TxCount},
};

use super::{ergo_client::ErgoClient, ergo_processor::ErgoProcessor};

pub struct ErgoBlockProvider {
    pub client: ErgoClient,
    pub processor: ErgoProcessor,
}

impl ErgoBlockProvider {
    pub fn new(node_url: Url, api_key: Option<&'static str>) -> Self {
        ErgoBlockProvider {
            client: ErgoClient { node_url, api_key },
            processor: ErgoProcessor {},
        }
    }
}

impl BlockProvider for ErgoBlockProvider {
    type InTx = Transaction;
    type OutTx = EuTx;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount) {
        self.processor.process_batch(block_batch, tx_count)
    }

    fn get_best_block(&self) -> Result<Block<Self::InTx>, String> {
        self.client.get_best_block()
    }

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::InTx>, String> {
        self.client.get_block_by_height(height)
    }

    fn get_processed_block_by_hash(&self, hash: BlockHash) -> Result<Block<Self::OutTx>, String> {
        let block = self.client.get_block_by_hash(hash)?;
        let processed_block = self.processor.process(&block);
        Ok(processed_block)
    }
}
