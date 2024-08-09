use pallas::ledger::traverse::MultiEraBlock;

use crate::{
    eutxo::eutxo_model::EuTx,
    model::{Block, BlockHash, BlockHeader, TxIndex},
};

use super::cardano_client::CBOR;

pub struct CardanoProcessor {}

impl CardanoProcessor {
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
            timestamp: (b.header().slot() as u32).into(),
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
                        tx_inputs: vec![],  // todo !!!
                        tx_outputs: vec![], // todo !!!
                    }
                })
                .collect(),
        ))
    }
}
