use crate::api::{BlockProcessor, TxIndex};
use crate::api::{CiBlock, CiIndexedTxid, CiTx, CiUtxo, Height};
use crate::log;
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use chrono::DateTime;

pub struct BtcProcessor;
impl BlockProcessor for BtcProcessor {
    type Block = bitcoin::Block;
    fn process(&self, block_batch: Vec<(Height, Self::Block, usize)>) -> Vec<CiBlock> {
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
                        ci_block.hash
                    );
                }
                ci_block
            })
            .collect()
    }
}

impl From<(Height, bitcoin::Block, usize)> for CiBlock {
    fn from(block: (Height, bitcoin::Block, usize)) -> Self {
        CiBlock {
            hash: block.1.block_hash().to_string(),
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

impl From<(TxIndex, bitcoin::Transaction)> for CiTx {
    fn from(tx: (TxIndex, bitcoin::Transaction)) -> Self {
        CiTx {
            is_coinbase: tx.1.is_coinbase(),
            tx_id: tx.1.compute_txid().to_byte_array(), //TODO make this more efficient
            tx_index: tx.0,
            ins: tx
                .1
                .input
                .iter()
                .map(|input| CiIndexedTxid {
                    tx_id: input.previous_output.txid.to_byte_array(),
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
                    let script_hash: [u8; 32] =
                        sha256::Hash::hash(out.script_pubkey.as_bytes()).to_byte_array();
                    CiUtxo {
                        index: out_index as u16,
                        address,
                        script_hash,
                        value: out.value.to_sat(),
                    }
                })
                .collect(),
        }
    }
}
