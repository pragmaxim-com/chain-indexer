use crate::{
    api::{BlockProcessor, BlockchainClient, BlockProvider},
    eutxo::eutxo_model::EuTx,
    model::{Block, BlockHash, BlockHeight, TxCount},
};

use super::{btc_client::BtcClient, btc_processor::BtcProcessor};

pub struct BtcBlockProvider {
    pub client: BtcClient,
    pub processor: BtcProcessor,
}

impl BtcBlockProvider {
    pub fn new(api_host: &str, api_username: &str, api_password: &str) -> Self {
        BtcBlockProvider {
            client: BtcClient::new(api_host, api_username, api_password),
            processor: BtcProcessor {},
        }
    }
}

impl BlockProvider for BtcBlockProvider {
    type InTx = bitcoin::Transaction;
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