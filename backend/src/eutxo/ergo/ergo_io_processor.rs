use crate::api::IoProcessor;
use crate::eutxo::eutxo_model;
use crate::eutxo::eutxo_model::{Address, Asset, AssetAction, AssetName, AssetPointer, BlockHeight, Box, InputPointer, InputRef, PolicyId, TxPointer, Utxo, UtxoPointer};
use crate::model::{AssetType, BoxWeight};
use ergo_lib::{
    ergotree_ir::{
        chain::{
            address,
            ergo_box::{BoxId, ErgoBox},
            token::TokenId,
        },
        serialization::SigmaSerializable,
    },
    wallet::box_selector::ErgoBoxAssets,
};
use redb::ReadTransaction;
use redbit::IndexedPointer;
use redbit::*;
use crate::info;

pub struct ErgoIoProcessor {}

impl IoProcessor<BoxId, InputRef, ErgoBox, Utxo> for ErgoIoProcessor {
    fn process_inputs(&self, ins: &[BoxId], read_tx: &ReadTransaction) -> Vec<InputRef> {
        ins.iter()
            .map(|input| {
                let box_id_slice: &[u8] = input.as_ref();
                let box_id_bytes: Vec<u8> = box_id_slice.into();
                let box_id = eutxo_model::BoxId(box_id_bytes);
                let utxos =
                    Box::get_by_box_id(read_tx, &box_id)
                        .expect("Failed to get Utxo by ErgoBox");
                match utxos.first() {
                    Some(first_utxo) => {
                        InputRef {
                            id: InputPointer::from_parent(first_utxo.id.parent.clone(), first_utxo.id.index())
                        }
                    }
                    None => {
                        InputRef {
                            id: InputPointer::from_parent(TxPointer::from_parent(BlockHeight(0), 0), 0)
                        }
                    }
                }
            })
            .collect()
    }

    fn process_outputs(&self, outs: &[ErgoBox], tx_pointer: TxPointer) -> (BoxWeight, Vec<Utxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        let mut asset_count = 0;
        for (out_index, out) in outs.iter().enumerate() {
            let box_id = out.box_id();
            let box_id_slice: &[u8] = box_id.as_ref();
            let box_id_bytes: Vec<u8> = box_id_slice.into();
            let ergo_tree_opt = out.ergo_tree.sigma_serialize_bytes().ok();
            let ergo_tree_t8_opt = out.ergo_tree.template_bytes().ok();
            let address_opt = address::Address::recreate_from_ergo_tree(&out.ergo_tree)
                .map(|a| a.content_bytes())
                .ok();

            let utxo_pointer = UtxoPointer::from_parent(tx_pointer.clone(), out_index as u16);
            let address = Address(address_opt.map(|a| a.to_vec()).unwrap_or_else(|| vec![]));
            let amount = *out.value.as_u64();
            let ergo_box = Some(Box {
                id: utxo_pointer.clone(),
                box_id: eutxo_model::BoxId(box_id_bytes.clone()),
                tree: eutxo_model::Tree(ergo_tree_opt.unwrap_or(vec![])),
                tree_t8: eutxo_model::TreeT8(ergo_tree_t8_opt.unwrap_or(vec![])),
            });

            let assets: Vec<Asset> =
                if let Some(assets) = out.tokens() {
                    let mut result = Vec::with_capacity(assets.len());
                    for (index, asset) in assets.enumerated() {
                        let asset_id: Vec<u8> = asset.token_id.into();
                        let amount = asset.amount;
                        let amount_u64: u64 = amount.into();
                        let is_mint = outs.first().is_some_and(|o| {
                            let new_token_id: TokenId = o.box_id().into();
                            new_token_id == asset.token_id
                        });

                        let action = match is_mint {
                            true => AssetType::Mint, // TODO!! for Minting it might not be enough to check first boxId
                            _ => AssetType::Transfer,
                        };
                        let asset_pointer = AssetPointer::from_parent(utxo_pointer.clone(), index as u8);
                        result.push(Asset {
                            id: asset_pointer,
                            name: AssetName(asset_id),
                            amount: amount_u64,
                            policy_id: PolicyId(vec![]),
                            asset_action: AssetAction(action.into()),
                        });
                    }
                    result
                } else {
                    vec![]
                };


            asset_count += assets.len();
            result_outs.push(Utxo {
                id: utxo_pointer.clone(),
                assets,
                address,
                amount,
                ergo_box,
            })
        }
        (asset_count + result_outs.len(), result_outs)
    }
}
