use ergo_lib::{
    ergotree_ir::{
        chain::{address::Address, ergo_box::ErgoBox},
        serialization::SigmaSerializable,
    },
    wallet::box_selector::ErgoBoxAssets,
};

use crate::{
    api::OutputProcessor,
    eutxo::{eutxo_model::EuUtxo, eutxo_schema::DbSchema},
};

const DB_INDEX_ADDRESS: String = "ADDRESS".to_string();
const DB_INDEX_ERGO_TREE_HASH: String = "ERGO_TREE_HASH".to_string();
const DB_INDEX_ERGO_TREE_T8_HASH: String = "ERGO_TREE_T8_HASH".to_string();

pub struct ErgoOutputProcessor {
    pub db_index_manager: DbSchema,
}

impl OutputProcessor<ErgoBox, EuUtxo> for ErgoOutputProcessor {
    fn process_output(&self, out_index: usize, out: &ErgoBox) -> EuUtxo {
        let ergo_tree_opt = out.ergo_tree.sigma_serialize_bytes().ok();
        let ergo_tree_t8_opt = out.ergo_tree.template_bytes().ok();
        let address_opt = Address::recreate_from_ergo_tree(&out.ergo_tree)
            .map(|a| a.content_bytes())
            .ok();

        let mut o2m_db_indexes = Vec::with_capacity(2);
        if let Some(index_number) = self
            .db_index_manager
            .db_index_table
            .one_to_many
            .get(&DB_INDEX_ERGO_TREE_HASH)
        {
            if let Some(ergo_tree) = ergo_tree_opt {
                o2m_db_indexes.push((*index_number, ergo_tree));
            }
        }

        if let Some(index_number) = self
            .db_index_manager
            .db_index_table
            .one_to_many
            .get(&DB_INDEX_ERGO_TREE_T8_HASH)
        {
            if let Some(ergo_tree_t8) = ergo_tree_t8_opt {
                o2m_db_indexes.push((*index_number, ergo_tree_t8));
            }
        }

        if let Some(index_number) = self
            .db_index_manager
            .db_index_table
            .one_to_many
            .get(&DB_INDEX_ADDRESS)
        {
            if let Some(address) = address_opt {
                o2m_db_indexes.push((*index_number, address));
            }
        }

        let tokens = out.tokens();
        let mut result = Vec::with_capacity(tokens.len());
        if let Some(assets) = tokens {
            for token in assets {
                let policy_id = token.policy().to_vec();

                for asset in token.assets() {
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
        }

        EuUtxo {
            utxo_index: (out_index as u16).into(),
            o2m_db_indexes,
            o2o_db_indexes: EMPTY_VEC,
            assets: result,
            utxo_value: (*out.value.as_u64()).into(),
        }
    }
}
