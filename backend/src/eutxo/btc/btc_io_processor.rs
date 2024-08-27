use crate::api::IoProcessor;
use crate::eutxo::eutxo_model::{EuTxInput, EuUtxo, TxHashWithIndex};
use crate::eutxo::eutxo_schema::DbSchema;
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;
use model::{AssetAction, AssetId, AssetValue, BoxWeight, O2mIndexValue};

pub const EMPTY_ASSETS_VEC: Vec<(AssetId, AssetValue, AssetAction)> = Vec::new();

pub struct BtcIoProcessor {
    pub db_schema: DbSchema,
}

impl BtcIoProcessor {
    pub fn new(db_schema: DbSchema) -> Self {
        BtcIoProcessor { db_schema }
    }
}

impl IoProcessor<bitcoin::TxIn, EuTxInput, bitcoin::TxOut, EuUtxo> for BtcIoProcessor {
    fn process_inputs(&self, ins: &[bitcoin::TxIn]) -> Vec<EuTxInput> {
        ins.iter()
            .map(|input| {
                EuTxInput::TxHashInput(TxHashWithIndex {
                    tx_hash: input.previous_output.txid.to_byte_array().into(),
                    utxo_index: (input.previous_output.vout as u16).into(),
                })
            })
            .collect()
    }
    fn process_outputs(&self, outs: &[bitcoin::TxOut]) -> (BoxWeight, Vec<EuUtxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = if let Ok(address) =
                Address::from_script(out.script_pubkey.as_script(), Network::Bitcoin)
            {
                Some(address.to_string().into_bytes())
            } else {
                out.script_pubkey.p2pk_public_key().map(|pk| {
                    Address::p2pkh(pk.pubkey_hash(), Network::Bitcoin)
                        .to_string()
                        .into_bytes()
                })
            };
            let script_hash: O2mIndexValue = sha256::Hash::hash(out.script_pubkey.as_bytes())
                .as_byte_array()
                .to_vec()
                .into();

            let mut o2m_db_indexes: Vec<(u8, O2mIndexValue)> = Vec::with_capacity(2);

            if let Some(index_number) = self.db_schema.o2m_index_number_by_name.get("SCRIPT_HASH") {
                o2m_db_indexes.push((*index_number, script_hash));
            }

            if let Some(index_number) = self.db_schema.o2m_index_number_by_name.get("ADDRESS") {
                if let Some(address) = address_opt {
                    o2m_db_indexes.push((*index_number, address.into()));
                }
            }

            result_outs.push(EuUtxo {
                utxo_index: (out_index as u16).into(),
                o2m_db_indexes,
                o2o_db_indexes: vec![],
                assets: EMPTY_ASSETS_VEC,
                utxo_value: out.value.to_sat().into(),
            })
        }
        (result_outs.len(), result_outs)
    }
}
