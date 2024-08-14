use crate::{
    api::OutputProcessor,
    eutxo::{eutxo_model::EuUtxo, eutxo_schema::DbSchema},
    model::AssetAction,
};
use pallas::{codec::minicbor::Encode, codec::minicbor::Encoder, ledger::traverse::MultiEraOutput};

const DB_INDEX_ADDRESS: String = "ADDRESS".to_string();
const DB_INDEX_SCRIPT_HASH: String = "SCRIPT_HASH".to_string();

pub struct CardanoOutputProcessor {
    pub db_schema: DbSchema,
}

impl CardanoOutputProcessor {
    pub fn new(db_schema: DbSchema) -> Self {
        CardanoOutputProcessor { db_schema }
    }
}

impl OutputProcessor<MultiEraOutput<'_>, EuUtxo> for CardanoOutputProcessor {
    fn process_outputs(&self, outs: Vec<MultiEraOutput<'_>>) -> Vec<EuUtxo> {
        let result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = out.address().ok().map(|a| a.to_vec());
            let script_hash_opt = out.script_ref().map(|h| {
                let mut buffer = Vec::new();
                let mut encoder = Encoder::new(&mut buffer);
                let mut ctx = ();
                h.encode(&mut encoder, &mut ctx).unwrap();
                buffer
            });

            let mut o2m_db_indexes = Vec::with_capacity(2);

            if let Some(index_number) = self
                .db_schema
                .db_index_table
                .one_to_many
                .get(&DB_INDEX_SCRIPT_HASH)
            {
                if let Some(script_hash) = script_hash_opt {
                    o2m_db_indexes.push((*index_number, script_hash));
                }
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

            let assets = out.non_ada_assets();
            let mut result = Vec::with_capacity(assets.len());
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
                    result.push((asset_id, amount, action));
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
