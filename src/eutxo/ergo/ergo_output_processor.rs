use ergo_lib::{
    ergotree_ir::{
        chain::{address::Address, ergo_box::ErgoBox, token::TokenId},
        serialization::SigmaSerializable,
    },
    wallet::box_selector::ErgoBoxAssets,
};

use crate::{
    api::OutputProcessor,
    eutxo::{eutxo_model::EuUtxo, eutxo_schema::DbSchema},
    model::AssetAction,
};

const DB_INDEX_ADDRESS: String = "ADDRESS".to_string();
const DB_INDEX_ERGO_TREE_HASH: String = "ERGO_TREE_HASH".to_string();
const DB_INDEX_ERGO_TREE_T8_HASH: String = "ERGO_TREE_T8_HASH".to_string();
const DB_INDEX_BOX_ID: String = "BOX_ID".to_string();

pub struct ErgoOutputProcessor {
    pub db_index_manager: DbSchema,
}

impl OutputProcessor<ErgoBox, EuUtxo> for ErgoOutputProcessor {
    fn process_outputs(&self, outs: Vec<ErgoBox>) -> Vec<EuUtxo> {
        let result_outs = Vec::with_capacity(outs.len());
        for (out_index, out) in outs.iter().enumerate() {
            let box_id_slice: &[u8] = out.box_id().as_ref();
            let box_id: Vec<u8> = box_id_slice.into();
            let ergo_tree_opt = out.ergo_tree.sigma_serialize_bytes().ok();
            let ergo_tree_t8_opt = out.ergo_tree.template_bytes().ok();
            let address_opt = Address::recreate_from_ergo_tree(&out.ergo_tree)
                .map(|a| a.content_bytes())
                .ok();

            let mut o2o_db_indexes = Vec::with_capacity(2);
            if let Some(index_number) = self
                .db_index_manager
                .db_index_table
                .one_to_many
                .get(&DB_INDEX_BOX_ID)
            {
                o2o_db_indexes.push((*index_number, box_id));
            } else {
                panic!("Ergo BOX_ID index is missing in schema.yaml")
            }

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
            let mut result_assets = Vec::with_capacity(tokens.map_or_else(|| 0, |xs| xs.len()));
            if let Some(assets) = tokens {
                for asset in assets {
                    let asset_id: Vec<u8> = asset.token_id.into();
                    let amount = asset.amount;
                    let amount_i64: i64 = amount.into();
                    let amount_u64: u64 = amount.into();
                    let is_mint = outs.first().is_some_and(|o| {
                        let new_token_id: TokenId = o.box_id().into();
                        new_token_id == asset.token_id
                    });

                    let action = match (is_mint, amount_i64 < 0) {
                        (true, _) => AssetAction::Mint, // TODO!! for Minting it might not be enough to check first boxId
                        (_, true) => AssetAction::Burn, // TODO!! I don't know how burning works in ergo
                        _ => AssetAction::Transfer,
                    };
                    result_assets.push((asset_id, amount_u64, action));
                }
            }
            result_outs.push(EuUtxo {
                utxo_index: (out_index as u16).into(),
                o2m_db_indexes,
                o2o_db_indexes,
                assets: result_assets,
                utxo_value: (*out.value.as_u64()).into(),
            })
        }
        result_outs
    }
}
