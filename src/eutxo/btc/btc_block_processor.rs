use crate::api::{BlockProcessor, IoProcessor};
use crate::eutxo::eutxo_model::EuTx;
use crate::model::{AssetAction, AssetId, AssetValue, Block, TxCount, TxIndex};
use bitcoin_hashes::Hash;

use super::btc_io_processor::BtcIoProcessor;

pub const EMPTY_VEC: Vec<(AssetId, AssetValue, AssetAction)> = Vec::new();

// pub static GENESIS_START_TIME: u32 = 1231006505;

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Vec<u8>;

pub struct BtcBlockProcessor {
    pub io_processor: BtcIoProcessor,
}

impl BtcBlockProcessor {
    pub fn new(io_processor: BtcIoProcessor) -> Self {
        BtcBlockProcessor { io_processor }
    }

    fn process_tx(&self, tx_index: &TxIndex, tx: &bitcoin::Transaction) -> EuTx {
        EuTx {
            tx_hash: tx.compute_txid().to_byte_array().into(),
            tx_index: tx_index.clone(),
            tx_inputs: self.io_processor.process_inputs(&tx.input),
            tx_outputs: self.io_processor.process_outputs(&tx.output),
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
