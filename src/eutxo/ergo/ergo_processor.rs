use ergo_lib::chain::transaction::Transaction;

use crate::{
    api::BlockProcessor,
    eutxo::eutxo_model::EuTx,
    model::{Block, TxCount},
};

use super::ergo_config::ErgoIndexes;

pub type OutputAddress = Vec<u8>;
pub type OutputErgoTreeHash = Vec<u8>;
pub type OutputErgoTreeT8Hash = Vec<u8>;

pub struct ErgoProcessor {
    pub indexes: ErgoIndexes,
}

impl ErgoProcessor {
    pub fn new(indexes: ErgoIndexes) -> Self {
        ErgoProcessor { indexes }
    }
}

impl BlockProcessor for ErgoProcessor {
    type InTx = Transaction;
    type OutTx = EuTx;

    fn process_block(&self, btc_block: &Block<Self::InTx>) -> Block<Self::OutTx> {
        btc_block.into()
    }

    fn process_batch(
        &self,
        block_batch: &Vec<Block<Self::InTx>>,
        tx_count: TxCount,
    ) -> (Vec<Block<Self::OutTx>>, TxCount) {
        (
            block_batch
                .into_iter()
                .map(|btc_block| {
                    let eu_block: Block<Self::OutTx> = btc_block.into();
                    eu_block
                })
                .collect(),
            tx_count,
        )
    }
}

impl From<&Block<Transaction>> for Block<EuTx> {
    fn from(block: &Block<Transaction>) -> Self {
        Block::new(
            block.header.clone(),
            vec![],
            /*             block
                           .txs
                           .iter()
                           .enumerate()
                           .map(|(tx_index, tx)| (&(tx_index as u16).into(), tx).into())
                           .collect(),
            */
        )
    }
}
/*
impl From<(&TxIndex, &Transaction)> for EuTx {
    fn from(tx: (&TxIndex, &Transaction)) -> Self {
        let tx_id: [u8; 32] = tx.1.id().0 .0;
        EuTx {
            tx_hash: tx_id.into(),
            tx_index: tx.0.clone(),
            tx_inputs: tx
                .1
                .inputs
                .iter()
                .map(|input| EuTxInput {
                    tx_hash: input.previous_output.txid.to_byte_array().into(),
                    utxo_index: (input.previous_output.vout as u16).into(),
                })
                .collect(),
            tx_outputs: tx
                .1
                .output
                .iter()
                .enumerate()
                .map(|(out_index, out)| {
                    let address = if let Ok(address) =
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

                    let mut db_indexes = Vec::with_capacity(2); // Pre-allocate capacity for 2 elements
                    db_indexes.push((0, script_hash));
                    if let Some(address) = address {
                        db_indexes.push((1, address));
                    }

                    EuUtxo {
                        utxo_index: (out_index as u16).into(),
                        db_indexes,
                        assets: EMPTY_VEC,
                        utxo_value: out.value.to_sat().into(),
                    }
                })
                .collect(),
        }
    }
}
 */
