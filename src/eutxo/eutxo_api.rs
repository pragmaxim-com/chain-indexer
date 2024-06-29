use std::borrow::Cow;

use crate::api::{AssetId, AssetValue, BlockHash, BlockHeight, BlockTimestamp, TxHash, TxIndex};

pub type UtxoIndex = u16;
pub type UtxoValue = u64;

pub type UtxoIndexName = Cow<'static, str>;
pub type UtxoIndexValue = Vec<u8>;

#[derive(Debug, Clone)]
pub struct CiUtxo {
    pub index: UtxoIndex,
    pub db_indexes: Vec<(UtxoIndexName, UtxoIndexValue)>,
    pub assets: Vec<(AssetId, AssetValue)>,
    pub value: UtxoValue,
}

#[derive(Debug, Clone)]
pub struct CiIndexedTxid {
    pub tx_hash: TxHash,
    pub utxo_index: UtxoIndex,
}

#[derive(Debug, Clone)]
pub struct CiTx {
    pub is_coinbase: bool,
    pub tx_hash: TxHash,
    pub tx_index: TxIndex,
    pub ins: Vec<CiIndexedTxid>,
    pub outs: Vec<CiUtxo>,
}

#[derive(Debug, Clone)]
pub struct CiBlock {
    pub hash: BlockHash,
    pub height: BlockHeight,
    pub time: BlockTimestamp,
    pub txs: Vec<CiTx>,
}
