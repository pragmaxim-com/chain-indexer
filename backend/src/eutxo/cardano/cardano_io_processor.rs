pub use redbit::*;

use crate::api::IoProcessor;
use crate::eutxo::eutxo_model::{
    Address, Asset, AssetAction, AssetName, AssetPointer, InputPointer, InputRef, PolicyId,
    Transaction, TxHash, TxPointer, Utxo, UtxoPointer,
};
use crate::model::{AssetType, BoxWeight};
use pallas::{
    codec::minicbor::{Encode, Encoder},
    ledger::traverse::{MultiEraInput, MultiEraOutput},
};
use redb::ReadTransaction;

pub struct CardanoIoProcessor {}

impl IoProcessor<MultiEraInput<'_>, InputRef, MultiEraOutput<'_>, Utxo> for CardanoIoProcessor {
    fn process_inputs(&self, ins: &[MultiEraInput<'_>], tx: &ReadTransaction) -> Vec<InputRef> {
        // iter zipped with index
        ins.iter()
            .map(|input| {
                let tx_hash: [u8; 32] = **input.hash();
                let tx_pointers = Transaction::get_ids_by_hash(tx, &TxHash(tx_hash))
                    .expect("Failed to get Transaction by TxHash");
                let tx_pointer = tx_pointers.first().expect("Failed to get Transaction pointer");
                InputRef {
                    id: InputPointer::from_parent(tx_pointer.clone(), input.index() as u16),
                }
            })
            .collect()
    }

    fn process_outputs(
        &self,
        outs: &[MultiEraOutput<'_>],
        tx_pointer: TxPointer,
    ) -> (BoxWeight, Vec<Utxo>) {
        let mut result_outs = Vec::with_capacity(outs.len());
        let mut asset_count = 0;
        let mut ctx = ();
        for (out_index, out) in outs.iter().enumerate() {
            let address_opt = out.address().ok().map(|a| a.to_vec());
            let script_hash_opt = out.script_ref().map(|h| {
                let mut buffer = Vec::new();
                let mut encoder = Encoder::new(&mut buffer);
                h.encode(&mut encoder, &mut ctx).unwrap();
                buffer
            });
            let utxo_pointer = UtxoPointer::from_parent(tx_pointer.clone(), out_index as u16);

            let mut result_assets = Vec::with_capacity(out.non_ada_assets().iter().map(|p| p.assets().len()).sum());

            // start your pointer index at 0
            let mut idx: u8 = 0;

            for policy_assets in out.non_ada_assets() {
                // clone the policy‐id bytes once
                let pid_bytes = policy_assets.policy().to_vec();

                for asset in policy_assets.assets() {
                    let any_coin = asset.any_coin();
                    let action = match (asset.is_mint(), any_coin < 0) {
                        (true, _)   => AssetType::Mint,
                        (_, true)   => AssetType::Burn,
                        _           => AssetType::Transfer,
                    };

                    result_assets.push( Asset {
                        id:           AssetPointer::from_parent(utxo_pointer.clone(), idx),
                        amount:       any_coin.abs() as u64,
                        name:         AssetName(asset.name().to_vec()),
                        policy_id:    PolicyId(pid_bytes.clone()),
                        asset_action: AssetAction(action.into()),
                    });

                    idx += 1;
                }
            }

            asset_count += result_assets.len();
            result_outs.push(Utxo {
                id: utxo_pointer,
                amount: out.lovelace_amount().into(),
                address: Address(address_opt.unwrap_or_default()),
                assets: vec![], // TODO
                ergo_box: None,
            })
        }
        (asset_count + result_outs.len(), result_outs)
    }
}
