use crate::model::BatchWeight;
use actix_web::{HttpResponse, ResponseError};
use async_trait::async_trait;
use futures::Stream;
use hex::FromHexError;
use pallas::network::miniprotocols;
use redbit::AppError;
use serde::{Deserialize, Serialize};
use std::{fmt, pin::Pin};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainSyncError {
    pub error: String,
}
impl fmt::Display for ChainSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl ResponseError for ChainSyncError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::InternalServerError().json(self)
    }
}

impl From<redb::Error> for ChainSyncError {
    fn from(err: redb::Error) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}

impl From<reqwest::Error> for ChainSyncError {
    fn from(err: reqwest::Error) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<redb::TransactionError> for ChainSyncError {
    fn from(err: redb::TransactionError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<redb::CommitError> for ChainSyncError {
    fn from(err: redb::CommitError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<url::ParseError> for ChainSyncError {
    fn from(err: url::ParseError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<AppError> for ChainSyncError {
    fn from(err: AppError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<FromHexError> for ChainSyncError {
    fn from(err: FromHexError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<miniprotocols::chainsync::ClientError> for ChainSyncError {
    fn from(err: miniprotocols::chainsync::ClientError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<miniprotocols::blockfetch::ClientError> for ChainSyncError {
    fn from(err: miniprotocols::blockfetch::ClientError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<miniprotocols::localstate::ClientError> for ChainSyncError {
    fn from(err: miniprotocols::localstate::ClientError) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}
impl From<pallas::ledger::traverse::Error> for ChainSyncError {
    fn from(err: pallas::ledger::traverse::Error) -> Self {
        ChainSyncError::new(&err.to_string())
    }
}

impl ChainSyncError {
    pub fn new(error: &str) -> Self {
        ChainSyncError {
            error: error.to_string(),
        }
    }
}

pub trait BlockHeaderLike: Send + Sync + Clone {
    fn height(&self) -> u32;
    fn hash(&self) -> [u8; 32];
    fn prev_hash(&self) -> [u8; 32];
    fn timestamp(&self) -> u32;
}

pub trait BlockLike: Send + Sync {
    type Header: BlockHeaderLike + 'static;
    fn header(&self) -> &Self::Header;
    fn weight(&self) -> u32;
}

pub trait BlockProcessor<B: BlockLike> {
    type FromBlock: Send;

    fn process_block(&self, block: &Self::FromBlock) -> Result<B, ChainSyncError>;
}

pub trait BlockPersistence<B: BlockLike> {
    fn get_last_header(&self) -> Result<Option<B::Header>, ChainSyncError>;

    fn get_header_by_hash(&self, hash: [u8; 32]) -> Result<Vec<B::Header>, ChainSyncError>;

    fn store_blocks(&self, blocks: &Vec<Arc<B>>) -> Result<(), ChainSyncError>;

    fn update_blocks(&self, blocks: &Vec<Arc<B>>) -> Result<(), ChainSyncError>;
}

#[async_trait]
pub trait BlockProvider<B: BlockLike> {
    fn get_processed_block(&self, header: B::Header) -> Result<B, ChainSyncError>;

    async fn get_chain_tip(&self) -> Result<B::Header, ChainSyncError>;

    async fn stream(
        &self,
        chain_tip_header: B::Header,
        last_header: Option<B::Header>,
        min_batch_size: usize,
        fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<B>, BatchWeight)> + Send + 'life0>>;
}