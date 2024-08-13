use pallas::{codec::minicbor::Encode, codec::minicbor::Encoder, ledger::traverse::MultiEraBlock};

use super::{cardano_client::CBOR, cardano_config::CardanoIndexes};
use crate::{
    eutxo::eutxo_model::{EuTx, EuTxInput, EuUtxo},
    model::{AssetAction, AssetId, AssetValue, Block, BlockHash, BlockHeader, TxIndex},
    settings::Indexes,
};
pub const EMPTY_VEC: Vec<(AssetId, AssetValue)> = Vec::new();

pub type OutputAddress = Option<Vec<u8>>;
pub type OutputScriptHash = Option<Vec<u8>>;

pub static GENESIS_START_TIME: u32 = 1506203091;

pub struct CardanoProcessor {
    pub indexes: CardanoIndexes,
}

impl CardanoProcessor {
    pub fn new(indexes: CardanoIndexes) -> Self {
        CardanoProcessor { indexes }
    }

    pub fn process_block(&self, block: &CBOR) -> Result<Block<EuTx>, String> {
        let b = MultiEraBlock::decode(block).map_err(|e| e.to_string())?;

        let hash: [u8; 32] = *b.header().hash();
        let prev_h = b
            .header()
            .previous_hash()
            .unwrap_or(pallas::crypto::hash::Hash::new([0u8; 32]));
        let prev_hash: [u8; 32] = *prev_h;
        let header = BlockHeader {
            height: (b.header().number() as u32).into(),
            timestamp: (b.header().slot() as u32 + GENESIS_START_TIME).into(),
            hash: BlockHash(hash),
            prev_hash: BlockHash(prev_hash),
        };

        Ok(Block::new(
            header,
            b.txs()
                .iter()
                .enumerate()
                .map(|(tx_index, tx)| {
                    let tx_hash: [u8; 32] = *tx.hash();
                    EuTx {
                        tx_hash: tx_hash.into(),
                        tx_index: TxIndex(tx_index as u16),
                        tx_inputs: tx
                            .inputs()
                            .iter()
                            .map(|input| {
                                let tx_hash: [u8; 32] = **input.hash();
                                EuTxInput {
                                    tx_hash: tx_hash.into(),
                                    utxo_index: (input.index() as u16).into(),
                                }
                            })
                            .collect(),
                        tx_outputs: tx
                            .outputs()
                            .iter()
                            .enumerate()
                            .map(|(out_index, out)| {
                                let address_opt = out.address().ok().map(|a| a.to_vec());
                                let script_hash_opt = out.script_ref().map(|h| {
                                    let mut buffer = Vec::new();
                                    let mut encoder = Encoder::new(&mut buffer);
                                    let mut ctx = ();
                                    h.encode(&mut encoder, &mut ctx).unwrap();
                                    buffer
                                });

                                let db_indexes =
                                    self.indexes.create_indexes((address_opt, script_hash_opt));

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

                                EuUtxo {
                                    utxo_index: (out_index as u16).into(),
                                    db_indexes,
                                    assets: result,
                                    utxo_value: out.lovelace_amount().into(),
                                }
                            })
                            .collect(),
                    }
                })
                .collect(),
        ))
    }
}
