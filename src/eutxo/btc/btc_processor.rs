use std::borrow::Cow;

use crate::api::BlockProcessor;
use crate::eutxo::eutxo_model::{EuTx, EuTxInput, EuUtxo};
use crate::model::{AssetId, AssetValue, Block, TxCount, TxIndex};
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

// define constant for address and script_hash
pub const ADDRESS_INDEX: &str = "address";
pub const SCRIPT_HASH_INDEX: &str = "script_hash";

pub const EMPTY_VEC: Vec<(AssetId, AssetValue)> = Vec::new();

pub struct BtcProcessor;
impl BlockProcessor for BtcProcessor {
    type InTx = bitcoin::Transaction;
    type OutTx = EuTx;

    fn process(&self, btc_block: &Block<Self::InTx>) -> Block<Self::OutTx> {
        btc_block.into()
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
                    let eu_block: Block<Self::OutTx> = btc_block.into();
                    eu_block
                })
                .collect(),
            tx_count,
        )
    }
}

impl From<&Block<bitcoin::Transaction>> for Block<EuTx> {
    fn from(block: &Block<bitcoin::Transaction>) -> Self {
        Block::new(
            block.header,
            block
                .txs
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| (&(tx_index as u16).into(), tx).into())
                .collect(),
        )
    }
}

impl From<(&TxIndex, &bitcoin::Transaction)> for EuTx {
    fn from(tx: (&TxIndex, &bitcoin::Transaction)) -> Self {
        EuTx {
            is_coinbase: tx.1.is_coinbase(),
            tx_hash: tx.1.compute_txid().to_byte_array().into(),
            tx_index: tx.0.clone(),
            ins: tx
                .1
                .input
                .iter()
                .map(|input| EuTxInput {
                    tx_hash: input.previous_output.txid.to_byte_array().into(),
                    utxo_index: (input.previous_output.vout as u16).into(),
                })
                .collect(),
            outs: tx
                .1
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
                            bitcoin::Address::p2pkh(pk.pubkey_hash(), bitcoin::Network::Bitcoin)
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

                    let mut db_indexes = Vec::with_capacity(2); // Pre-allocate capacity for 2 elements
                    db_indexes.push((Cow::Borrowed(SCRIPT_HASH_INDEX), script_hash));
                    if let Some(address) = address {
                        db_indexes.push((Cow::Borrowed(ADDRESS_INDEX), address));
                    }

                    EuUtxo {
                        index: (out_index as u16).into(),
                        db_indexes,
                        assets: EMPTY_VEC,
                        value: out.value.to_sat().into(),
                    }
                })
                .collect(),
        }
    }
}
