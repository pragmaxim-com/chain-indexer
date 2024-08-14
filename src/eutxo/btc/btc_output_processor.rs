use crate::api::OutputProcessor;
use crate::eutxo::eutxo_model::EuUtxo;
use crate::eutxo::eutxo_schema::DbSchema;
use crate::model::{AssetAction, AssetId, AssetValue};
use bitcoin::{Address, Network};
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

pub const EMPTY_ASSETS_VEC: Vec<(AssetId, AssetValue, AssetAction)> = Vec::new();

const DB_INDEX_ADDRESS: String = "ADDRESS".to_string();
const DB_INDEX_SCRIPT_HASH: String = "SCRIPT_HASH".to_string();

pub struct BtcOutputProcessor {
    pub db_schema: DbSchema,
}

impl BtcOutputProcessor {
    pub fn new(db_schema: DbSchema) -> Self {
        BtcOutputProcessor { db_schema }
    }
}

impl OutputProcessor<bitcoin::TxOut, EuUtxo> for BtcOutputProcessor {
    fn process_outputs(&self, outs: Vec<bitcoin::TxOut>) -> Vec<EuUtxo> {
        let result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = if let Ok(address) =
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

            let mut o2m_db_indexes = Vec::with_capacity(2);

            if let Some(index_number) = self
                .db_schema
                .db_index_table
                .one_to_many
                .get(&DB_INDEX_SCRIPT_HASH)
            {
                o2m_db_indexes.push((*index_number, script_hash));
            }

            if let Some(index_number) = self
                .db_schema
                .db_index_table
                .one_to_many
                .get(&DB_INDEX_ADDRESS)
            {
                if let Some(address) = address_opt {
                    o2m_db_indexes.push((*index_number, address));
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
        result_outs
    }
}
