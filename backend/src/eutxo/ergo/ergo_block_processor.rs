use redbit::*;
use ergo_lib::{chain::block::FullBlock, wallet::signing::ErgoTransaction};
use redb::ReadTransaction;
use super::ergo_io_processor::ErgoIoProcessor;
use crate::model::TxCount;
use crate::api::{BlockProcessor, IoProcessor, ServiceError};
use crate::eutxo::eutxo_model::{Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, Transaction, TxHash, TxPointer};

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

    fn process_batch(&self, block_batch: &[Self::FromBlock], tx_count: TxCount, read_tx: &ReadTransaction) -> Result<(Vec<Block>, TxCount), ServiceError> {
        let blocks: Result<Vec<Block>, ServiceError> = block_batch
            .iter()
            .map(|btc_block| self.process_block(btc_block, read_tx))
            .collect();
        blocks.map(|blocks| (blocks, tx_count))
    }

    fn process_block(&self, b: &Self::FromBlock, read_tx: &ReadTransaction) -> Result<Block, ServiceError> {
        let mut block_weight: usize = 0;
        let mut result_txs = Vec::with_capacity(b.block_transactions.transactions.len());

        let block_hash: [u8; 32] = b.header.id.0.into();
        let prev_block_hash: [u8; 32] = b.header.parent_id.0.into();

        let id = BlockHeight(b.header.height);
        let header = BlockHeader {
            id: id.clone(),
            timestamp: BlockTimestamp((b.header.timestamp / 1000) as u32),
            hash: BlockHash(block_hash),
            prev_hash: BlockHash(prev_block_hash),
        };
        
        for (tx_index, tx) in b.block_transactions.transactions.iter().enumerate() {
            let tx_hash: [u8; 32] = tx.id().0.0;
            let tx_id = TxPointer::from_parent(header.id.clone(), tx_index as u16);
            let inputs = self.io_processor.process_inputs(&tx.inputs_ids().to_vec(), read_tx);
            let (box_weight, outputs) = self.io_processor.process_outputs(&tx.outputs().to_vec(), tx_id.clone()); //TODO perf check
            block_weight += box_weight;
            block_weight += inputs.len();
            result_txs.push(Transaction {
                id: tx_id.clone(),
                hash: TxHash(tx_hash),
                utxos: outputs,
                inputs
            })
        }

        Ok(Block  { id: id.clone(), header, transactions: result_txs, weight: block_weight as u16 })
    }
}
