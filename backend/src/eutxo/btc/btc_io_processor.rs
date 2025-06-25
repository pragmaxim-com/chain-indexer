use crate::api::IoProcessor;
use crate::eutxo::eutxo_model::{Address, BlockHeight, InputPointer, InputRef, Transaction, TxHash, TxPointer, Utxo, UtxoPointer};
use crate::model::BoxWeight;
use bitcoin_hashes::Hash;
use redb::ReadTransaction;
pub use redbit::*;
use crate::info;

pub struct BtcIoProcessor { }

impl IoProcessor<bitcoin::TxIn, InputRef, bitcoin::TxOut, Utxo> for BtcIoProcessor {
    fn process_inputs(&self, ins: &[bitcoin::TxIn], tx: &ReadTransaction) -> Vec<InputRef> {
        ins.iter()
            .map(|input| {
                let tx_hash = TxHash(input.previous_output.txid.to_byte_array());
                let txs = Transaction::get_by_hash(tx, &tx_hash).expect("Failed to get Transaction by TxHash");

                match txs.first() {
                    Some(first_tx) => {
                        InputRef {
                            id: InputPointer::from_parent(first_tx.id.clone(), input.previous_output.vout as u16),
                        }
                    }
                    None => {
                        info!("Tx {:?} not found, it should be coinbase", tx_hash.clone());
                        InputRef {
                            id: InputPointer::from_parent(TxPointer::from_parent(BlockHeight(0), 0), 0)
                        }
                    }
                }

            })
            .collect()
    }
    fn process_outputs(&self, outs: &[bitcoin::TxOut], tx_pointer: TxPointer) -> (BoxWeight, Vec<Utxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = if let Ok(address) =
                bitcoin::Address::from_script(out.script_pubkey.as_script(), bitcoin::Network::Bitcoin)
            {
                Some(address.to_string().into_bytes())
            } else {
                out.script_pubkey.p2pk_public_key().map(|pk| {
                    bitcoin::Address::p2pkh(pk.pubkey_hash(), bitcoin::Network::Bitcoin)
                        .to_string()
                        .into_bytes()
                })
            };
            let script_hash = out.script_pubkey.as_bytes().to_vec();


            result_outs.push(Utxo {
                id: UtxoPointer::from_parent(tx_pointer.clone(), out_index as u16),
                amount: out.value.to_sat().into(),
                address: Address(address_opt.unwrap_or_default()),
                ergo_box: None, // TODO: Handle ErgoBox if needed
                assets: vec![], // TODO

            })
        }
        (result_outs.len(), result_outs)
    }
}
