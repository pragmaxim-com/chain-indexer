use crate::model::{
    AssetId, AssetValue, Block, BlockHeader, DbIndexName, DbIndexValue, TxCount, TxHash, TxIndex,
};
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

#[derive(Debug, Clone)]
pub struct EuBlock {
    pub header: BlockHeader,
    pub txs: Vec<EuTx>,
}

impl Block for EuBlock {
    fn header(&self) -> BlockHeader {
        self.header
    }

    fn tx_count(&self) -> TxCount {
        self.txs.len()
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
