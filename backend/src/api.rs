use std::{fmt, pin::Pin};

use crate::{
    codec_tx::TxPkBytes,
    eutxo::{eutxo_codec_utxo::UtxoPkBytes, eutxo_schema::DbSchema},
    rocks_db_batch::{CustomFamilies, Families},
};
use actix_web::{HttpResponse, ResponseError};
use async_trait::async_trait;
use bitcoin::block::Bip34Error;
use futures::Stream;
use hex::FromHexError;
use lru::LruCache;
use model::{
    AssetId, BatchWeight, Block, BlockHeader, BlockHeight, BoxWeight, O2mIndexValue, O2oIndexValue,
    TxCount, TxHash,
};
use pallas::network::miniprotocols;
use rocksdb::{MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceError {
    pub error: String,
}
// Implement `ResponseError` for `ServiceError`
impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().json(self)
    }
}
impl From<rocksdb::Error> for ServiceError {
    fn from(err: rocksdb::Error) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<reqwest::Error> for ServiceError {
    fn from(err: reqwest::Error) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<url::ParseError> for ServiceError {
    fn from(err: url::ParseError) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<FromHexError> for ServiceError {
    fn from(err: FromHexError) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<miniprotocols::chainsync::ClientError> for ServiceError {
    fn from(err: miniprotocols::chainsync::ClientError) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<miniprotocols::blockfetch::ClientError> for ServiceError {
    fn from(err: miniprotocols::blockfetch::ClientError) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<miniprotocols::localstate::ClientError> for ServiceError {
    fn from(err: miniprotocols::localstate::ClientError) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<bitcoincore_rpc::Error> for ServiceError {
    fn from(err: bitcoincore_rpc::Error) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<pallas::ledger::traverse::Error> for ServiceError {
    fn from(err: pallas::ledger::traverse::Error) -> Self {
        ServiceError::new(&err.to_string())
    }
}
impl From<Bip34Error> for ServiceError {
    fn from(err: Bip34Error) -> Self {
        ServiceError::new(&err.to_string())
    }
}

impl ServiceError {
    pub fn new(error: &str) -> Self {
        ServiceError {
            error: error.to_string(),
        }
    }
}

pub trait BlockProcessor {
    type FromBlock: Send;
    type IntoTx: Send;

    fn process_block(&self, block: &Self::FromBlock) -> Result<Block<Self::IntoTx>, ServiceError>;

    fn process_batch(
        &self,
        block_batch: &[Self::FromBlock],
        tx_count: TxCount,
    ) -> Result<(Vec<Block<Self::IntoTx>>, TxCount), ServiceError>;
}

pub trait IoProcessor<FromInput, IntoInput, FromOutput, IntoOutput> {
    fn process_inputs(&self, ins: &[FromInput]) -> Vec<IntoInput>;
    fn process_outputs(&self, outs: &[FromOutput]) -> (BoxWeight, Vec<IntoOutput>);
}

#[async_trait]
pub trait BlockProvider {
    type OutTx: Send;

    fn get_schema(&self) -> DbSchema;

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block<Self::OutTx>, ServiceError>;

    async fn get_chain_tip(&self) -> Result<BlockHeader, ServiceError>;

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<Self::OutTx>>, BatchWeight)> + Send + 'life0>>;
}

pub trait TxReadService: Sync + Sync {
    type CF: CustomFamilies;
    type Tx;

    fn get_txs_by_height(&self, block_height: &BlockHeight) -> Result<Vec<Self::Tx>, ServiceError>;
}

pub trait TxWriteService: Sync + Sync {
    type CF: CustomFamilies;
    type Tx;

    #[warn(clippy::too_many_arguments)]
    fn persist_txs(
        &self,
        block: &Block<Self::Tx>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        families: &Families<Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<Self::CF>,
    ) -> Result<(), rocksdb::Error>;
}

pub trait BlockMonitor<Tx> {
    fn monitor(&self, block_batch: &[Block<Tx>], batch_weight: &BatchWeight);
}
