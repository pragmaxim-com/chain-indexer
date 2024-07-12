use crate::api::{
    AssetId, AssetValue, Block, BlockHash, BlockHeight, BlockTimestamp, DbIndexName, DbIndexValue,
    TxCount, TxHash, TxIndex,
};

pub type InputIndex = u16;

pub type UtxoIndex = u16;
pub type UtxoValue = u64;

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
    pub hash: BlockHash,
    pub parent_hash: BlockHash,
    pub height: BlockHeight,
    pub timestamp: BlockTimestamp,
    pub txs: Vec<EuTx>,
}

impl Block for EuBlock {
    fn hash(&self) -> BlockHash {
        self.hash
    }

    fn prev_hash(&self) -> BlockHash {
        self.parent_hash
    }

    fn height(&self) -> BlockHeight {
        self.height
    }

    fn timestamp(&self) -> BlockTimestamp {
        self.timestamp
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
