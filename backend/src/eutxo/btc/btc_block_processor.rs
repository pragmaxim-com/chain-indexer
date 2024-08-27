use super::btc_io_processor::BtcIoProcessor;
use crate::api::{BlockProcessor, IoProcessor, ServiceError};
use crate::eutxo::eutxo_model::EuTx;
use bitcoin_hashes::Hash;
use model::{AssetAction, AssetId, AssetValue, Block, BlockHeader, TxCount, TxIndex};

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
        let (_, outputs) = self.io_processor.process_outputs(&tx.output);
        EuTx {
            tx_hash: tx.compute_txid().to_byte_array().into(),
            tx_index: *tx_index,
            tx_inputs: self.io_processor.process_inputs(&tx.input),
            tx_outputs: outputs,
        }
    }
}

impl BlockProcessor for BtcBlockProcessor {
    type FromBlock = bitcoin::Block;
    type IntoTx = EuTx;

    fn process_block(&self, block: &Self::FromBlock) -> Result<Block<Self::IntoTx>, ServiceError> {
        let height = block.bip34_block_height()?;
        let header = BlockHeader {
            height: (height as u32).into(),
            timestamp: block.header.time.into(),
            hash: block.block_hash().to_byte_array().into(),
            prev_hash: block.header.prev_blockhash.to_byte_array().into(),
        };

        let mut block_weight = 0;
        Ok(Block::new(
            header,
            block
                .txdata
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    block_weight += tx.input.len() + tx.output.len();
                    self.process_tx(&(tx_index as u16).into(), tx)
                })
                .collect(),
            block_weight,
        ))
    }

    fn process_batch(
        &self,
        block_batch: &[Self::FromBlock],
        tx_count: TxCount,
    ) -> Result<(Vec<Block<Self::IntoTx>>, TxCount), ServiceError> {
        let blocks: Result<Vec<Block<Self::IntoTx>>, ServiceError> = block_batch
            .iter()
            .map(|btc_block| self.process_block(btc_block))
            .collect();
        blocks.map(|blocks| (blocks, tx_count))
    }
}
