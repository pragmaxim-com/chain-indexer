use ergo_lib::{chain::transaction::Transaction, wallet::signing::ErgoTransaction};

use crate::{
    api::{BlockProcessor, OutputProcessor},
    eutxo::eutxo_model::{EuTx, EuTxInput},
    model::{Block, OutputId, TxCount, TxIndex},
};

use super::ergo_output_processor::ErgoOutputProcessor;

pub type OutputAddress = Vec<u8>;
pub type OutputErgoTreeHash = Vec<u8>;
pub type OutputErgoTreeT8Hash = Vec<u8>;

pub struct ErgoBlockProcessor {
    pub output_processor: ErgoOutputProcessor,
}

impl ErgoBlockProcessor {
    pub fn new(output_processor: ErgoOutputProcessor) -> Self {
        ErgoBlockProcessor { output_processor }
    }
}

impl BlockProcessor for ErgoBlockProcessor {
    type FromTx = Transaction;
    type IntoTx = EuTx;

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::FromTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::IntoTx>>, TxCount) {
        (
            block_batch
                .into_iter()
                .map(|btc_block| {
                    let eu_block: Block<Self::IntoTx> = self.process_block(btc_block);
                    eu_block
                })
                .collect(),
            tx_count,
        )
    }

    fn process_block(&self, b: &Block<Self::FromTx>) -> Block<Self::IntoTx> {
        Block::new(
            b.header.clone(),
            b.txs
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    let tx_hash: [u8; 32] = tx.id().0 .0;
                    EuTx {
                        tx_hash: tx_hash.into(),
                        tx_index: TxIndex(tx_index as u16),
                        tx_inputs: tx
                            .inputs_ids()
                            .iter()
                            .map(|input| {
                                let box_id_slice: &[u8] = input.as_ref();
                                let box_id: [u8; 32] = box_id_slice
                                    .try_into()
                                    .expect("slice with incorrect length");
                                EuTxInput::BoxIdInput(OutputId(box_id))
                            })
                            .collect(),
                        tx_outputs: self.output_processor.process_outputs(tx.outputs().to_vec()), //TODO perf check
                    }
                })
                .collect(),
        )
    }
}
