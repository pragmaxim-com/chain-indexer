use crate::api::BlockProcessor;
use crate::eutxo::eutxo_model::{EuTx, EuTxInput, EuUtxo};
use crate::model::{AssetAction, AssetId, AssetValue, Block, TxCount, TxIndex};
use crate::settings::Indexes;
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

use super::btc_config::BitcoinIndexes;

pub const EMPTY_VEC: Vec<(AssetId, AssetValue, AssetAction)> = Vec::new();

pub static GENESIS_START_TIME: u32 = 1231006505;

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Vec<u8>;

pub struct BtcProcessor {
    pub indexes: BitcoinIndexes,
}

impl BtcProcessor {
    pub fn new(indexes: BitcoinIndexes) -> Self {
        BtcProcessor { indexes }
    }

    fn process_tx(&self, tx_index: &TxIndex, tx: &bitcoin::Transaction) -> EuTx {
        EuTx {
            tx_hash: tx.compute_txid().to_byte_array().into(),
            tx_index: tx_index.clone(),
            tx_inputs: tx
                .input
                .iter()
                .map(|input| EuTxInput {
                    tx_hash: input.previous_output.txid.to_byte_array().into(),
                    utxo_index: (input.previous_output.vout as u16).into(),
                })
                .collect(),
            tx_outputs: tx
                .output
                .iter()
                .enumerate()
                .map(|(out_index, out)| {
                    let address = if let Ok(address) =
                        Address::from_script(out.script_pubkey.as_script(), Network::Bitcoin)
                    {
                        Some(address.to_string().into_bytes())
                    } else if let Some(pk) = out.script_pubkey.p2pk_public_key() {
                        Some(
                            Address::p2pkh(pk.pubkey_hash(), Network::Bitcoin)
                                .to_string()
                                .into_bytes(),
                        )
                    } else if out.script_pubkey.is_op_return() {
                        None
                    } else {
                        None
                    };
                    let script_hash = sha256::Hash::hash(out.script_pubkey.as_bytes())
                        .as_byte_array()
                        .to_vec();

                    let db_indexes = self.indexes.create_indexes((address, script_hash));

                    EuUtxo {
                        utxo_index: (out_index as u16).into(),
                        db_indexes,
                        assets: EMPTY_VEC,
                        utxo_value: out.value.to_sat().into(),
                    }
                })
                .collect(),
        }
    }
}

impl BlockProcessor for BtcProcessor {
    type InTx = bitcoin::Transaction;
    type OutTx = EuTx;

    fn process_block(&self, btc_block: &Block<Self::InTx>) -> Block<Self::OutTx> {
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
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount) {
        (
            block_batch
                .into_iter()
                .map(|btc_block| {
                    let eu_block: Block<Self::OutTx> = self.process_block(btc_block);
                    eu_block
                })
                .collect(),
            tx_count,
        )
    }
}
