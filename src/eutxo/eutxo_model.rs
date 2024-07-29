use crate::model::{
    AssetId, AssetValue, DbIndexCfIndex, DbIndexValue, Transaction, TxHash, TxIndex,
};
use derive_more::{AsRef, Display, From, Into};

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct InputIndex(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoValue(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoIndex(pub u16);

#[derive(Debug, Clone)]
pub struct EuUtxo {
    pub utxo_index: UtxoIndex,
    pub db_indexes: Vec<(DbIndexCfIndex, DbIndexValue)>,
    pub assets: Vec<(AssetId, AssetValue)>,
    pub utxo_value: UtxoValue,
}

#[derive(Debug, Clone)]
pub struct EuTxInput {
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

impl Transaction for EuTx {
    fn is_coinbase(&self) -> bool {
        self.tx_index.0 == 0
    }

    fn hash(&self) -> &TxHash {
        &self.tx_hash
    }

    fn index(&self) -> &TxIndex {
        &self.tx_index
    }
}

pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const ASSETS_BY_UTXO_PK_CF: &str = "ASSETS_BY_UTXO_PK_CF";
pub const ASSET_ID_BY_ASSET_BIRTH_PK_CF: &str = "ASSET_ID_BY_ASSET_BIRTH_PK_CF";
pub const ASSET_BIRTH_PK_BY_ASSET_ID_CF: &str = "ASSET_BIRTH_PK_BY_ASSET_ID_CF";
pub const ASSET_BIRTH_PK_WITH_ASSET_PK_CF: &str = "ASSET_BIRTH_PK_WITH_ASSET_PK_CF";

pub fn get_eutxo_column_families() -> Vec<&'static str> {
    vec![
        UTXO_VALUE_BY_PK_CF,
        UTXO_PK_BY_INPUT_PK_CF,
        ASSETS_BY_UTXO_PK_CF,
        ASSET_ID_BY_ASSET_BIRTH_PK_CF,
        ASSET_BIRTH_PK_BY_ASSET_ID_CF,
        ASSET_BIRTH_PK_WITH_ASSET_PK_CF,
    ]
}
