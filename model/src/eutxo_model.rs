use derive_more::{AsRef, Display, From, Into};
use serde::{Deserialize, Serialize};

use crate::{AssetAction, AssetId, AssetValue, O2mIndexValue, O2oIndexValue, TxHash, TxIndex};

pub type DbIndexNumber = u8;
pub type DbIndexValueSize = u16;
pub type DbIndexName = String;
pub type DbIndexEnabled = bool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct InputIndex(u16);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoValue(pub u64);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoIndex(pub u16);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EuUtxo {
    pub utxo_index: UtxoIndex,
    pub o2m_db_indexes: Vec<(DbIndexNumber, O2mIndexValue)>,
    pub o2o_db_indexes: Vec<(DbIndexNumber, O2oIndexValue)>,
    pub assets: Vec<(AssetId, AssetValue, AssetAction)>,
    pub utxo_value: UtxoValue,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EuTxInput {
    TxHashInput(TxHashWithIndex),
    OutputIndexInput(DbIndexNumber, O2oIndexValue),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxHashWithIndex {
    pub tx_hash: TxHash,
    pub utxo_index: UtxoIndex,
}

#[derive(Debug, Clone)]
pub struct EuTx {
    pub tx_hash: TxHash,
    pub tx_index: TxIndex,
    pub tx_inputs: Vec<EuTxInput>,
    pub tx_outputs: Vec<EuUtxo>,
}