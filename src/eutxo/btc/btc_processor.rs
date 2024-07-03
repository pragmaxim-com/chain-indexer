use std::borrow::Cow;

use crate::api::{
    AssetId, AssetValue, BlockHeight, BlockProcessor, BlockTimestamp, TxCount, TxIndex,
};
use crate::eutxo::eutxo_api::{EuBlock, EuTx, EuTxInput, EuUtxo};
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

// define constant for address and script_hash
pub const ADDRESS_INDEX: &str = "address";
pub const SCRIPT_HASH_INDEX: &str = "script_hash";

pub const EMPTY_VEC: Vec<(AssetId, AssetValue)> = Vec::new();

pub struct BtcProcessor;
impl BlockProcessor for BtcProcessor {
    type InBlock = bitcoin::Block;
    type OutBlock = EuBlock;
    fn process(
        &self,
        block_batch: &Vec<(BlockHeight, Self::InBlock, TxCount, BlockTimestamp)>,
    ) -> Vec<EuBlock> {
        block_batch
            .into_iter()
            .map(|height_block| {
                let eu_block: EuBlock = height_block.into();
                eu_block
            })
            .collect()
    }
}

impl From<&(BlockHeight, bitcoin::Block, TxCount, BlockTimestamp)> for EuBlock {
    fn from(block: &(BlockHeight, bitcoin::Block, TxCount, BlockTimestamp)) -> Self {
        EuBlock {
            hash: block.1.block_hash().to_byte_array(),
            time: block.1.header.time as i64,
            height: block.0,
            txs: block
                .1
                .txdata
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| (&(tx_index as u16), tx).into())
                .collect(),
        }
    }
}

impl From<(&TxIndex, &bitcoin::Transaction)> for EuTx {
    fn from(tx: (&TxIndex, &bitcoin::Transaction)) -> Self {
        EuTx {
            is_coinbase: tx.1.is_coinbase(),
            tx_hash: tx.1.compute_txid().to_byte_array(),
            tx_index: *tx.0,
            ins: tx
                .1
                .input
                .iter()
                .map(|input| EuTxInput {
                    tx_hash: input.previous_output.txid.to_byte_array(),
                    utxo_index: input.previous_output.vout as u16,
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
                        index: out_index as u16,
                        db_indexes,
                        assets: EMPTY_VEC,
                        value: out.value.to_sat(),
                    }
                })
                .collect(),
        }
    }
}
