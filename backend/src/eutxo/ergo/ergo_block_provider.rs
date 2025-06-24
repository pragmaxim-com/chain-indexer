use std::{pin::Pin, str::FromStr, sync::Arc};
use redbit::*;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use min_batch::ext::MinBatchExt;
use redb::ReadTransaction;
use reqwest::Url;

use crate::model::{BatchWeight};
use crate::{
    api::{BlockProcessor, BlockProvider, ServiceError},
    info,
    settings::ErgoConfig,
};
use crate::eutxo::eutxo_model::{Block, BlockHeader, BlockHeight};
use super::{
    ergo_block_processor::ErgoBlockProcessor, ergo_client::ErgoClient,
    ergo_io_processor::ErgoIoProcessor,
};

pub struct ErgoBlockProvider {
    pub client: Arc<ErgoClient>,
    pub processor: Arc<ErgoBlockProcessor>,
    pub db: Arc<redb::Database>,
}

impl ErgoBlockProvider {
    pub fn new(ergo_config: &ErgoConfig, db: Arc<redb::Database>) -> Self {
        ErgoBlockProvider {
            client: Arc::new(ErgoClient {
                node_url: Url::from_str(&ergo_config.api_host).unwrap(),
                api_key: ergo_config.api_key.clone(),
            }),
            processor: Arc::new(ErgoBlockProcessor::new(ErgoIoProcessor{})),
            db
        }
    }
}

#[async_trait]
impl BlockProvider for ErgoBlockProvider {

    fn get_processed_block(&self, header: BlockHeader, read_tx: &ReadTransaction) -> Result<Block, ServiceError> {
        let block = self.client.get_block_by_hash_sync(header.hash)?;
        self.processor.process_block(&block, read_tx)
    }

    async fn get_chain_tip(&self, read_tx: &ReadTransaction) -> Result<BlockHeader, ServiceError> {
        self.client
            .get_best_block_async()
            .await
            .and_then(|b| self.processor.process_block(&b, read_tx))
            .map(|b| b.header)
    }

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
        fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block>, BatchWeight)> + Send + 'life0>> {
        let read_tx = self.db.begin_read().unwrap();
        let best_header = self.get_chain_tip(&read_tx).await.unwrap();
        let last_height = last_header.map_or(1, |h| h.id.0);
        info!("Indexing from {} to {}", last_height, best_header.id.0);
        let heights = last_height..=best_header.id.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn(async move {
                    client
                        .get_block_by_height_async(BlockHeight(height))
                        .await
                        .unwrap()
                })
            })
            .buffered(fetching_par)
            .map(|res| match res {
                Ok(block) => {
                    let processor = Arc::clone(&self.processor);
                    let db = Arc::clone(&self.db);
                    let read_tx = db.begin_read().unwrap();
                    tokio::task::spawn_blocking(move || processor.process_block(&block, &read_tx).unwrap())
                }
                Err(e) => panic!("Error: {:?}", e),
            })
            .buffered(processing_par)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.weight as usize)
            .boxed()
    }
}
