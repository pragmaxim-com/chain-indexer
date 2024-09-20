use ergo_lib::{
    ergotree_ir::{
        chain::{
            address::Address,
            ergo_box::{BoxId, ErgoBox},
            token::TokenId,
        },
        serialization::SigmaSerializable,
    },
    wallet::box_selector::ErgoBoxAssets,
};

use crate::{
    api::IoProcessor,
    eutxo::{
        eutxo_model::{EuTxInput, EuUtxo},
        eutxo_schema::DbSchema,
    },
};
use model::{
    eutxo_model::DbIndexNumber, AssetAction, AssetId, AssetValue, BoxWeight, O2mIndexValue,
    O2oIndexValue,
};

pub struct ErgoIoProcessor {
    pub db_schema: DbSchema,
    pub box_id_index_number: DbIndexNumber,
}

impl ErgoIoProcessor {
    pub fn new(db_schema: DbSchema) -> Self {
        let schema = db_schema.clone();
        ErgoIoProcessor {
            db_schema,
            box_id_index_number: schema
                .clone()
                .o2o_index_number_by_name
                .get("BOX_ID")
                .unwrap()
                .to_owned(),
        }
    }
}

impl IoProcessor<BoxId, EuTxInput, ErgoBox, EuUtxo> for ErgoIoProcessor {
    fn process_inputs(&self, ins: &[BoxId]) -> Vec<EuTxInput> {
        ins.iter()
            .map(|input| {
                let box_id_slice: &[u8] = input.as_ref();
                EuTxInput::OutputIndexInput(
                    self.box_id_index_number,
                    O2oIndexValue(box_id_slice.to_vec()),
                )
            })
            .collect()
    }

    fn process_outputs(&self, outs: &[ErgoBox]) -> (BoxWeight, Vec<EuUtxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        let mut asset_count = 0;
        for (out_index, out) in outs.iter().enumerate() {
            let box_id = out.box_id();
            let box_id_slice: &[u8] = box_id.as_ref();
            let box_id_bytes: Vec<u8> = box_id_slice.into();
            let ergo_tree_opt = out.ergo_tree.sigma_serialize_bytes().ok();
            let ergo_tree_t8_opt = out.ergo_tree.template_bytes().ok();
            let address_opt = Address::recreate_from_ergo_tree(&out.ergo_tree)
                .map(|a| a.content_bytes())
                .ok();

            let mut o2o_db_indexes: Vec<(DbIndexNumber, O2oIndexValue)> = Vec::with_capacity(2);
            if let Some(index_number) = self.db_schema.o2o_index_number_by_name.get("BOX_ID") {
                o2o_db_indexes.push((*index_number, box_id_bytes.into()));
            } else {
                panic!("Ergo BOX_ID index is missing in schema.yaml")
            }

            let mut o2m_db_indexes: Vec<(DbIndexNumber, O2mIndexValue)> = Vec::with_capacity(3);
            if let Some(index_number) = self
                .db_schema
                .o2m_index_number_by_name
                .get("ERGO_TREE_HASH")
            {
                if let Some(ergo_tree) = ergo_tree_opt {
                    o2m_db_indexes.push((*index_number, ergo_tree.into()));
                }
            }

            if let Some(index_number) = self
                .db_schema
                .o2m_index_number_by_name
                .get("ERGO_TREE_T8_HASH")
            {
                if let Some(ergo_tree_t8) = ergo_tree_t8_opt {
                    o2m_db_indexes.push((*index_number, ergo_tree_t8.into()));
                }
            }

            if let Some(index_number) = self.db_schema.o2m_index_number_by_name.get("ADDRESS") {
                if let Some(address) = address_opt {
                    o2m_db_indexes.push((*index_number, address.into()));
                }
            }

            let result_assets: Vec<(AssetId, AssetValue, AssetAction)> =
                if let Some(assets) = out.tokens() {
                    let mut result = Vec::with_capacity(assets.len());
                    for asset in assets {
                        let asset_id: Vec<u8> = asset.token_id.into();
                        let amount = asset.amount;
                        let amount_u64: u64 = amount.into();
                        let is_mint = outs.first().is_some_and(|o| {
                            let new_token_id: TokenId = o.box_id().into();
                            new_token_id == asset.token_id
                        });

                        let action = match is_mint {
                            true => AssetAction::Mint, // TODO!! for Minting it might not be enough to check first boxId
                            _ => AssetAction::Transfer,
                        };
                        result.push((asset_id.into(), amount_u64, action));
                    }
                    result
                } else {
                    vec![]
                };

            asset_count += result_assets.len();
            result_outs.push(EuUtxo {
                utxo_index: (out_index as u16).into(),
                o2m_db_indexes,
                o2o_db_indexes,
                assets: result_assets,
                utxo_value: (*out.value.as_u64()).into(),
            })
        }
        (asset_count + result_outs.len(), result_outs)
    }
}
