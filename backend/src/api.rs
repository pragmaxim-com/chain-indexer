
use crate::model::{BatchWeight, BoxWeight, TxCount, };
use std::{fmt, pin::Pin};
use actix_web::{HttpResponse, ResponseError};
use async_trait::async_trait;
use futures::Stream;
use hex::FromHexError;
use pallas::network::miniprotocols;
use redb::ReadTransaction;
use redbit::AppError;
use serde::{Deserialize, Serialize};
use crate::eutxo::eutxo_model::{Block, BlockHeader, TxPointer};

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

impl From<redb::Error> for ServiceError {
    fn from(err: redb::Error) -> Self {
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
impl From<AppError> for ServiceError {
    fn from(err: AppError) -> Self {
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
impl From<pallas::ledger::traverse::Error> for ServiceError {
    fn from(err: pallas::ledger::traverse::Error) -> Self {
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

    fn process_block(&self, block: &Self::FromBlock, read_tx: &ReadTransaction) -> Result<Block, ServiceError>;

}

pub trait IoProcessor<FromInput, IntoInput, FromOutput, IntoOutput> {
    fn process_inputs(&self, ins: &[FromInput], tx: &ReadTransaction) -> Vec<IntoInput>;
    fn process_outputs(&self, outs: &[FromOutput], tx_pointer: TxPointer) -> (BoxWeight, Vec<IntoOutput>);
}

#[async_trait]
pub trait BlockProvider {

    fn get_processed_block(&self, header: BlockHeader, read_tx: &ReadTransaction) -> Result<Block, ServiceError>;

    async fn get_chain_tip(&self, read_tx: &ReadTransaction) -> Result<BlockHeader, ServiceError>;

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
        fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block>, BatchWeight)> + Send + 'life0>>;
}

pub trait BlockMonitor {
    fn monitor(&self, block_batch: &[Block], batch_weight: &BatchWeight);
}
