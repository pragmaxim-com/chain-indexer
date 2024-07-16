use crate::model::{AssetId, AssetValue, DbIndexName, DbIndexValue, Transaction, TxHash, TxIndex};
use derive_more::{AsRef, Display, From, Into};

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct InputIndex(u16);

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoValue(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct UtxoIndex(pub u16);

#[derive(Debug, Clone)]
pub struct EuUtxo {
    pub index: UtxoIndex,
    pub db_indexes: Vec<(DbIndexName, DbIndexValue)>,
    pub assets: Vec<(AssetId, AssetValue)>,
    pub value: UtxoValue,
}

#[derive(Debug, Clone)]
pub struct EuTxInput {
    pub tx_hash: TxHash,
    pub utxo_index: UtxoIndex,
}

#[derive(Debug, Clone)]
pub struct EuTx {
    pub is_coinbase: bool,
    pub tx_hash: TxHash,
    pub tx_index: TxIndex,
    pub ins: Vec<EuTxInput>,
    pub outs: Vec<EuUtxo>,
}

impl Transaction for EuTx {
    fn is_coinbase(&self) -> bool {
        self.is_coinbase
    }

    fn hash(&self) -> &TxHash {
        &self.tx_hash
    }

    fn index(&self) -> &TxIndex {
        &self.tx_index
    }
}

pub const BLOCK_HASH_BY_PK_CF: &str = "BLOCK_HASH_BY_PK_CF";
pub const BLOCK_PK_BY_HASH_CF: &str = "BLOCK_PK_BY_HASH_CF";
pub const TX_HASH_BY_PK_CF: &str = "TX_HASH_BY_PK_CF";
pub const TX_PK_BY_HASH_CF: &str = "TX_PK_BY_HASH_CF";
pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const META_CF: &str = "META_CF";

pub fn get_eutxo_column_families() -> Vec<&'static str> {
    vec![
        META_CF,
        BLOCK_HASH_BY_PK_CF,
        BLOCK_PK_BY_HASH_CF,
        TX_HASH_BY_PK_CF,
        TX_PK_BY_HASH_CF,
        UTXO_VALUE_BY_PK_CF,
        UTXO_PK_BY_INPUT_PK_CF,
    ]
}
