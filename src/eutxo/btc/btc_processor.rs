use std::borrow::Cow;

use crate::api::{BlockHeight, BlockProcessor, TxIndex};
use crate::eutxo::eutxo_api::{CiBlock, CiTx, CiTxInput, CiUtxo, UtxoIndexValue};
use crate::log;
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use chrono::DateTime;

// define constant for address and script_hash
pub const ADDRESS_INDEX: &str = "address";
pub const SCRIPT_HASH_INDEX: &str = "script_hash";

pub struct BtcProcessor;
impl BlockProcessor for BtcProcessor {
    type InBlock = bitcoin::Block;
    type OutBlock = CiBlock;
    fn process(&self, block_batch: Vec<(BlockHeight, Self::InBlock, usize)>) -> Vec<CiBlock> {
        block_batch
            .into_iter()
            .map(|height_block| {
                let ci_block: CiBlock = height_block.into();
                if ci_block.height % 1000 == 0 {
                    let datetime = DateTime::from_timestamp(ci_block.time as i64, 0).unwrap();
                    let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                    log!(
                        "Block @ {} : {} : {}",
                        ci_block.height,
                        readable_date,
                        hex::encode(&ci_block.hash)
                    );
                }
                ci_block
            })
            .collect()
    }
}

impl From<(BlockHeight, bitcoin::Block, usize)> for CiBlock {
    fn from(block: (BlockHeight, bitcoin::Block, usize)) -> Self {
        CiBlock {
            hash: block.1.block_hash().as_byte_array().to_vec(),
            time: block.1.header.time as i64,
            height: block.0,
            txs: block
                .1
                .txdata
                .into_iter()
                .enumerate()
                .map(|(tx_index, tx)| (tx_index as u16, tx).into())
                .collect(),
        }
    }
}

fn get_indexes(
    address: Option<UtxoIndexValue>,
    script_hash: UtxoIndexValue,
) -> Vec<(Cow<'static, str>, UtxoIndexValue)> {
    let mut vec = Vec::with_capacity(2); // Pre-allocate capacity for 2 elements
    vec.push((Cow::Borrowed(SCRIPT_HASH_INDEX), script_hash));
    if let Some(address) = address {
        vec.push((Cow::Borrowed(ADDRESS_INDEX), address));
    }
    vec
}

impl From<(TxIndex, bitcoin::Transaction)> for CiTx {
    fn from(tx: (TxIndex, bitcoin::Transaction)) -> Self {
        CiTx {
            is_coinbase: tx.1.is_coinbase(),
            tx_hash: tx.1.compute_txid().to_byte_array(),
            tx_index: tx.0,
            ins: tx
                .1
                .input
                .iter()
                .map(|input| CiTxInput {
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
                    let db_indexes = get_indexes(address, script_hash.to_vec());

                    CiUtxo {
                        index: out_index as u16,
                        db_indexes,
                        assets: vec![],
                        value: out.value.to_sat(),
                    }
                })
                .collect(),
        }
    }
}
