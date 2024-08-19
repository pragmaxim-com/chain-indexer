use ergo_lib::{chain::block::FullBlock, wallet::signing::ErgoTransaction};

use crate::{
    api::{BlockProcessor, IoProcessor},
    eutxo::eutxo_model::EuTx,
    model::{Block, BlockHeader, TxCount, TxIndex},
};

use super::ergo_io_processor::ErgoIoProcessor;

pub type OutputAddress = Vec<u8>;
pub type OutputErgoTreeHash = Vec<u8>;
pub type OutputErgoTreeT8Hash = Vec<u8>;

pub struct ErgoBlockProcessor {
    pub io_processor: ErgoIoProcessor,
}

impl ErgoBlockProcessor {
    pub fn new(io_processor: ErgoIoProcessor) -> Self {
        ErgoBlockProcessor { io_processor }
    }
}

impl BlockProcessor for ErgoBlockProcessor {
    type FromBlock = FullBlock;
    type IntoTx = EuTx;

    fn process_batch(
        &self,
        block_batch: &[Self::FromBlock],
        tx_count: TxCount,
    ) -> Result<(Vec<Block<Self::IntoTx>>, TxCount), String> {
        let blocks: Result<Vec<Block<Self::IntoTx>>, String> = block_batch
            .iter()
            .map(|btc_block| self.process_block(btc_block))
            .collect();
        blocks.map(|blocks| (blocks, tx_count))
    }

    fn process_block(&self, b: &Self::FromBlock) -> Result<Block<Self::IntoTx>, String> {
        let mut block_weight = 0;
        let mut result_txs = Vec::with_capacity(b.block_transactions.transactions.len());

        for (tx_index, tx) in b.block_transactions.transactions.iter().enumerate() {
            let tx_hash: [u8; 32] = tx.id().0 .0;
            let inputs = self.io_processor.process_inputs(&tx.inputs_ids().to_vec());
            let (box_weight, outputs) = self.io_processor.process_outputs(&tx.outputs().to_vec()); //TODO perf check
            block_weight += box_weight;
            block_weight += inputs.len();
            result_txs.push(EuTx {
                tx_hash: tx_hash.into(),
                tx_index: TxIndex(tx_index as u16),
                tx_inputs: inputs,
                tx_outputs: outputs, //TODO perf check
            })
        }
        let block_hash: [u8; 32] = b.header.id.0.into();
        let prev_block_hash: [u8; 32] = b.header.parent_id.0.into();

        let header = BlockHeader {
            height: b.header.height.into(),
            timestamp: ((b.header.timestamp / 1000) as u32).into(),
            hash: block_hash.into(),
            prev_hash: prev_block_hash.into(),
        };
        Ok(Block::new(header, result_txs, block_weight))
    }
}
