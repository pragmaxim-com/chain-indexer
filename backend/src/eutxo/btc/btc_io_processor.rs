use crate::api::IoProcessor;
use crate::eutxo::eutxo_model::{Address, InputPointer, InputRef, Transaction, TxHash, TxPointer, Utxo, UtxoPointer};
use crate::model::BoxWeight;
use bitcoin_hashes::Hash;
use redb::ReadTransaction;
pub use redbit::*;

pub struct BtcIoProcessor { }

impl IoProcessor<bitcoin::TxIn, InputRef, bitcoin::TxOut, Utxo> for BtcIoProcessor {
    fn process_inputs(&self, ins: &[bitcoin::TxIn], tx: &ReadTransaction) -> Vec<InputRef> {
        ins.iter()
            .map(|input| {
                let tx_hash = TxHash(input.previous_output.txid.to_byte_array());
                let tx_pointer = Transaction::get_by_hash(tx, &tx_hash.into()).unwrap().first().unwrap().clone().id;
                InputRef {
                    id: InputPointer::from_parent(tx_pointer, input.previous_output.vout as u16),
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
