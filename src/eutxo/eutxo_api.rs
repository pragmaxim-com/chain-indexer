use crate::api::{
    AssetId, AssetValue, BlockHash, BlockHeight, BlockTimestamp, DbIndexName, DbIndexValue, TxHash,
    TxIndex,
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
    pub height: BlockHeight,
    pub time: BlockTimestamp,
    pub txs: Vec<EuTx>,
}
