use pallas::ledger::traverse::MultiEraBlock;

use super::{cardano_client::CBOR, cardano_output_processor::CardanoOutputProcessor};
use crate::{
    api::OutputProcessor,
    eutxo::eutxo_model::{EuTx, EuTxInput, TxHashWithIndex},
    model::{AssetId, AssetValue, Block, BlockHash, BlockHeader, TxIndex},
};
pub const EMPTY_ASSETS_VEC: Vec<(AssetId, AssetValue)> = Vec::new();

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Option<Vec<u8>>;

pub static GENESIS_START_TIME: u32 = 1506203091;

pub struct CardanoBlockProcessor {
    pub output_processor: CardanoOutputProcessor,
}

impl CardanoBlockProcessor {
    pub fn new(output_processor: CardanoOutputProcessor) -> Self {
        CardanoBlockProcessor { output_processor }
    }

    pub fn process_block(&self, block: &CBOR) -> Result<Block<EuTx>, String> {
        let b = MultiEraBlock::decode(block).map_err(|e| e.to_string())?;

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

        Ok(Block::new(
            header,
            b.txs()
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    let tx_hash: [u8; 32] = *tx.hash();
                    EuTx {
                        tx_hash: tx_hash.into(),
                        tx_index: TxIndex(tx_index as u16),
                        tx_inputs: tx
                            .inputs()
                            .iter()
                            .map(|input| {
                                let tx_hash: [u8; 32] = **input.hash();
                                EuTxInput::TxHashInput(TxHashWithIndex {
                                    tx_hash: tx_hash.into(),
                                    utxo_index: (input.index() as u16).into(),
                                })
                            })
                            .collect(),
                        tx_outputs: self.output_processor.process_outputs(tx.outputs().to_vec()),
                    }
                })
                .collect(),
        ))
    }
}
