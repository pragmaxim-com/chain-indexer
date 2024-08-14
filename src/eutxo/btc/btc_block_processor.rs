use crate::api::{BlockProcessor, OutputProcessor};
use crate::eutxo::eutxo_model::{EuTx, EuTxInput, TxHashWithIndex};
use crate::model::{AssetAction, AssetId, AssetValue, Block, TxCount, TxIndex};
use bitcoin_hashes::Hash;

use super::btc_output_processor::BtcOutputProcessor;

pub const EMPTY_VEC: Vec<(AssetId, AssetValue, AssetAction)> = Vec::new();

// pub static GENESIS_START_TIME: u32 = 1231006505;

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Vec<u8>;

pub struct BtcBlockProcessor {
    pub output_processor: BtcOutputProcessor,
}

impl BtcBlockProcessor {
    pub fn new(output_processor: BtcOutputProcessor) -> Self {
        BtcBlockProcessor { output_processor }
    }

    fn process_tx(&self, tx_index: &TxIndex, tx: &bitcoin::Transaction) -> EuTx {
        EuTx {
            tx_hash: tx.compute_txid().to_byte_array().into(),
            tx_index: tx_index.clone(),
            tx_inputs: tx
                .input
                .iter()
                .map(|input| {
                    EuTxInput::TxHashInput(TxHashWithIndex {
                        tx_hash: input.previous_output.txid.to_byte_array().into(),
                        utxo_index: (input.previous_output.vout as u16).into(),
                    })
                })
                .collect(),
            tx_outputs: tx
                .output
                .iter()
                .enumerate()
                .map(|(out_index, out)| self.output_processor.process_output(out_index, out))
                .collect(),
        }
    }
}

impl BlockProcessor for BtcBlockProcessor {
    type FromTx = bitcoin::Transaction;
    type IntoTx = EuTx;

    fn process_block(&self, btc_block: &Block<Self::FromTx>) -> Block<Self::IntoTx> {
        Block::new(
            btc_block.header.clone(),
            btc_block
                .txs
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| self.process_tx(&(tx_index as u16).into(), tx))
                .collect(),
        )
    }

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
}
