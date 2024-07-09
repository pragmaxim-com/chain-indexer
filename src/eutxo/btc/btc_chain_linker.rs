use crate::{
    api::{BlockHash, BlockHeight, BlockProcessor, BlockchainClient, ChainLinker, TxCount},
    eutxo::eutxo_api::EuBlock,
};

use super::{
    btc_client::{BtcBlock, BtcClient},
    btc_processor::BtcProcessor,
};

pub struct BtcChainLinker {
    pub client: BtcClient,
    pub processor: BtcProcessor,
}

impl ChainLinker for BtcChainLinker {
    type InBlock = BtcBlock;

    type OutBlock = EuBlock;

    fn process_batch(
        &self,
        block_batch: &Vec<Self::InBlock>,
        tx_count: TxCount,
    ) -> (Vec<EuBlock>, TxCount) {
        self.processor.process_batch(block_batch, tx_count)
    }

    fn get_best_block(&self) -> Result<BtcBlock, String> {
        self.client.get_best_block()
    }

    fn get_block_by_height(&self, height: BlockHeight) -> Result<BtcBlock, String> {
        self.client.get_block_by_height(height)
    }

    fn get_processed_block_by_hash(&self, hash: BlockHash) -> Result<Self::OutBlock, String> {
        let block = self.client.get_block_by_hash(hash)?;
        let processed_block = self.processor.process(&block);
        Ok(processed_block)
    }
}
