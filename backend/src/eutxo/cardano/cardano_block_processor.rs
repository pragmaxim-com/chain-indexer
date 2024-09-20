use pallas::ledger::traverse::MultiEraBlock;

use super::{cardano_client::CBOR, cardano_io_processor::CardanoIoProcessor};
use crate::{
    api::{BlockProcessor, IoProcessor, ServiceError},
    eutxo::eutxo_model::EuTx,
};
use model::{AssetId, AssetValue, Block, BlockHash, BlockHeader, TxCount, TxIndex};

pub const EMPTY_ASSETS_VEC: Vec<(AssetId, AssetValue)> = Vec::new();

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
    type IntoTx = EuTx;

    fn process_block(&self, block: &CBOR) -> Result<Block<EuTx>, ServiceError> {
        let b = MultiEraBlock::decode(block)?;

        let hash: [u8; 32] = *b.header().hash();
        let prev_h = b
            .header()
            .previous_hash()
            .unwrap_or(pallas::crypto::hash::Hash::new([0u8; 32]));
        let prev_hash: [u8; 32] = *prev_h;
        let header = BlockHeader {
            height: (b.header().number() as u32).into(),
            timestamp: (b.header().slot() as u32 + GENESIS_START_TIME).into(),
            hash: BlockHash(hash),
            prev_hash: BlockHash(prev_hash),
        };

        let mut block_weight = 0;
        let txs: Vec<pallas::ledger::traverse::MultiEraTx> = b.txs();
        let mut result_txs = Vec::with_capacity(txs.len());

        for (tx_index, tx) in txs.iter().enumerate() {
            let tx_hash: [u8; 32] = *tx.hash();
            let inputs = self.io_processor.process_inputs(&tx.inputs());
            let (box_weight, outputs) = self.io_processor.process_outputs(&tx.outputs().to_vec()); //TODO perf check
            block_weight += box_weight;
            block_weight += inputs.len();
            result_txs.push(EuTx {
                tx_hash: tx_hash.into(),
                tx_index: TxIndex(tx_index as u16),
                tx_inputs: inputs,
                tx_outputs: outputs,
            })
        }

        Ok(Block::new(header, result_txs, block_weight))
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
