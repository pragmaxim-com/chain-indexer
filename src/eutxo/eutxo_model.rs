use crate::model::{
    AssetAction, AssetId, AssetValue, CompactionEnabled, O2mIndexValue, O2oIndexValue, TxHash,
    TxIndex,
};
use derive_more::{AsRef, Display, From, Into};

use super::eutxo_schema::DbIndexNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct InputIndex(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoValue(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoIndex(pub u16);

#[derive(Debug, Clone)]
pub struct EuUtxo {
    pub utxo_index: UtxoIndex,
    pub o2m_db_indexes: Vec<(DbIndexNumber, O2mIndexValue)>,
    pub o2o_db_indexes: Vec<(DbIndexNumber, O2oIndexValue)>,
    pub assets: Vec<(AssetId, AssetValue, AssetAction)>,
    pub utxo_value: UtxoValue,
}

#[derive(Debug, Clone)]
pub enum EuTxInput {
    TxHashInput(TxHashWithIndex),
    OutputIndexInput(DbIndexNumber, O2oIndexValue),
}

#[derive(Debug, Clone)]
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

pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const INPUT_PK_BY_UTXO_PK_CF: &str = "INPUT_PK_BY_UTXO_PK_CF";
pub const ASSET_BY_ASSET_PK_CF: &str = "ASSET_BY_ASSET_PK_CF";
pub const ASSET_ID_BY_ASSET_BIRTH_PK_CF: &str = "ASSET_ID_BY_ASSET_BIRTH_PK_CF";
pub const ASSET_BIRTH_PK_BY_ASSET_ID_CF: &str = "ASSET_BIRTH_PK_BY_ASSET_ID_CF";
pub const ASSET_BIRTH_PK_WITH_ASSET_PK_CF: &str = "ASSET_BIRTH_PK_WITH_ASSET_PK_CF";

pub fn get_eutxo_column_families() -> Vec<(&'static str, CompactionEnabled)> {
    vec![
        (UTXO_VALUE_BY_PK_CF, false),
        (UTXO_PK_BY_INPUT_PK_CF, false),
        (INPUT_PK_BY_UTXO_PK_CF, false),
        (ASSET_BY_ASSET_PK_CF, false),
        (ASSET_ID_BY_ASSET_BIRTH_PK_CF, false),
        (ASSET_BIRTH_PK_BY_ASSET_ID_CF, true),
        (ASSET_BIRTH_PK_WITH_ASSET_PK_CF, false),
    ]
}
