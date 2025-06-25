pub use redbit::*;

use super::{cardano_client::CBOR, cardano_io_processor::CardanoIoProcessor};
use crate::api::{BlockProcessor, IoProcessor, ServiceError};
use crate::eutxo::eutxo_model::{Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, Transaction, TxHash, TxPointer};
use crate::model::TxCount;
use pallas::ledger::traverse::MultiEraBlock;
use redb::ReadTransaction;

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Option<Vec<u8>>;

pub static GENESIS_START_TIME: u32 = 1506203091;

pub struct CardanoBlockProcessor {
    pub io_processor: CardanoIoProcessor,
}

impl CardanoBlockProcessor {
    pub fn new(io_processor: CardanoIoProcessor) -> Self {
        CardanoBlockProcessor { io_processor }
    }
}

impl BlockProcessor for CardanoBlockProcessor {
    type FromBlock = CBOR;

    fn process_block(&self, block: &CBOR, read_tx: &ReadTransaction) -> Result<Block, ServiceError> {
        let b = MultiEraBlock::decode(block)?;

        let hash: [u8; 32] = *b.header().hash();
        let prev_h = b
            .header()
            .previous_hash()
            .unwrap_or(pallas::crypto::hash::Hash::new([0u8; 32]));
        let prev_hash: [u8; 32] = *prev_h;
        let header = BlockHeader {
            id: BlockHeight(b.header().number() as u32),
            timestamp: BlockTimestamp(b.header().slot() as u32 + GENESIS_START_TIME),
            hash: BlockHash(hash),
            prev_hash: BlockHash(prev_hash),
        };

        let mut block_weight = 0;
        let txs: Vec<pallas::ledger::traverse::MultiEraTx> = b.txs();
        let mut result_txs = Vec::with_capacity(txs.len());

        for (tx_index, tx) in txs.iter().enumerate() {
            let tx_hash: [u8; 32] = *tx.hash();
            let tx_id = TxPointer::from_parent(header.id.clone(), tx_index as u16);
            let inputs = self.io_processor.process_inputs(&tx.inputs(), read_tx);
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

        Ok(Block { id: header.id.clone(), header, transactions: result_txs, weight: block_weight as u16 }) // usize
    }

}
