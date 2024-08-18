use crate::{
    api::IoProcessor,
    eutxo::{
        eutxo_model::{EuTxInput, EuUtxo, TxHashWithIndex},
        eutxo_schema::DbSchema,
    },
    model::{AssetAction, AssetId, AssetValue, O2mIndexValue},
};
use pallas::{
    codec::minicbor::{Encode, Encoder},
    ledger::traverse::{MultiEraInput, MultiEraOutput},
};

pub struct CardanoIoProcessor {
    pub db_schema: DbSchema,
}

impl CardanoIoProcessor {
    pub fn new(db_schema: DbSchema) -> Self {
        CardanoIoProcessor { db_schema }
    }
}

impl IoProcessor<MultiEraInput<'_>, EuTxInput, MultiEraOutput<'_>, EuUtxo> for CardanoIoProcessor {
    fn process_inputs(&self, ins: &Vec<MultiEraInput<'_>>) -> Vec<EuTxInput> {
        ins.iter()
            .map(|input| {
                let tx_hash: [u8; 32] = **input.hash();
                EuTxInput::TxHashInput(TxHashWithIndex {
                    tx_hash: tx_hash.into(),
                    utxo_index: (input.index() as u16).into(),
                })
            })
            .collect()
    }

    fn process_outputs(&self, outs: &Vec<MultiEraOutput<'_>>) -> Vec<EuUtxo> {
        let mut result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = out.address().ok().map(|a| a.to_vec());
            let script_hash_opt = out.script_ref().map(|h| {
                let mut buffer = Vec::new();
                let mut encoder = Encoder::new(&mut buffer);
                let mut ctx = ();
                h.encode(&mut encoder, &mut ctx).unwrap();
                buffer
            });

            let mut o2m_db_indexes: Vec<(u8, O2mIndexValue)> = Vec::with_capacity(2);

            if let Some(index_number) = self.db_schema.o2m_index_number_by_name.get("SCRIPT_HASH") {
                if let Some(script_hash) = script_hash_opt {
                    o2m_db_indexes.push((*index_number, script_hash.into()));
                }
            }

            if let Some(index_number) = self.db_schema.o2m_index_number_by_name.get("ADDRESS") {
                if let Some(address) = address_opt {
                    o2m_db_indexes.push((*index_number, address.into()));
                }
            }

            let assets = out.non_ada_assets();
            let mut result: Vec<(AssetId, AssetValue, AssetAction)> =
                Vec::with_capacity(assets.len());
            for policy_asset in assets {
                let policy_id = policy_asset.policy().to_vec();

                for asset in policy_asset.assets() {
                    let mut asset_id = policy_id.clone();
                    asset_id.extend(asset.name());

                    let any_coin = asset.any_coin();
                    let action = match (asset.is_mint(), any_coin < 0) {
                        (true, _) => AssetAction::Mint,
                        (_, true) => AssetAction::Burn,
                        _ => AssetAction::Transfer,
                    };
                    let amount = any_coin.abs() as u64;
                    result.push((asset_id.into(), amount, action));
                }
            }
            result_outs.push(EuUtxo {
                utxo_index: (out_index as u16).into(),
                o2m_db_indexes,
                o2o_db_indexes: vec![],
                assets: result,
                utxo_value: out.lovelace_amount().into(),
            })
        }
        result_outs
    }
}
