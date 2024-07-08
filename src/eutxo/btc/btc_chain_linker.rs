use crate::{
    api::{BlockProcessor, BlockchainClient, ChainLinker, TxCount},
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

    fn get_block(&self, height: u32) -> Result<BtcBlock, String> {
        self.client.get_block(height)
    }

    fn chain_link(&self, block: Self::OutBlock) -> Vec<Self::OutBlock> {
        todo!()
    }
}
