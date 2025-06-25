use super::btc_client::BtcBlock;
use super::btc_io_processor::BtcIoProcessor;
use crate::api::{BlockProcessor, IoProcessor, ServiceError};
use crate::eutxo::eutxo_model::{Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, Transaction, TxHash, TxPointer};
use redb::ReadTransaction;
pub use redbit::*;

// pub static GENESIS_START_TIME: u32 = 1231006505;

pub struct BtcBlockProcessor {
    pub io_processor: BtcIoProcessor,
}

impl BtcBlockProcessor {
    pub fn new(io_processor: BtcIoProcessor) -> Self {
        BtcBlockProcessor { io_processor }
    }

    fn process_tx(&self, height: BlockHeight, tx_index: u16, tx: &bitcoin::Transaction, read_tx: &ReadTransaction) -> Transaction {
        let tx_pointer = TxPointer::from_parent(height, tx_index);
        let (_, outputs) = self.io_processor.process_outputs(&tx.output, tx_pointer.clone());
        Transaction {
            id: tx_pointer.clone(),
            hash: TxHash(*tx.compute_txid().as_ref()),
            utxos: outputs,
            inputs: self.io_processor.process_inputs(&tx.input, read_tx),
        }
    }
}

impl BlockProcessor for BtcBlockProcessor {
    type FromBlock = BtcBlock;

    fn process_block(&self, block: &Self::FromBlock, read_tx: &ReadTransaction) -> Result<Block, ServiceError> {
        let header = BlockHeader {
            id: block.height.clone(),
            timestamp: BlockTimestamp(block.underlying.header.time),
            hash: BlockHash(*block.underlying.block_hash().as_ref()),
            prev_hash: BlockHash(*block.underlying.header.prev_blockhash.as_ref()),
        };

        let mut block_weight = 0;
        Ok(Block {
            id: block.height.clone(),
            header,
            transactions:
            block.underlying.txdata.iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    block_weight += tx.input.len() + tx.output.len();
                    self.process_tx(block.height.clone(), tx_index as u16, &tx, read_tx)
                }).collect(),
            weight: block_weight as u16 // TODO usize
        })
    }

}
