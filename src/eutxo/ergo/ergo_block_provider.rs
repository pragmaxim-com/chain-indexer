use std::{pin::Pin, str::FromStr, sync::Arc};

use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use min_batch::ext::MinBatchExt;
use reqwest::Url;

use crate::{
    api::{BlockProcessor, BlockProvider},
    eutxo::{eutxo_model::EuTx, eutxo_schema::DbSchema},
    info,
    model::{Block, BlockHeader, TxCount},
    settings::ErgoConfig,
};

use super::{
    ergo_block_processor::ErgoBlockProcessor, ergo_client::ErgoClient,
    ergo_io_processor::ErgoIoProcessor,
};

pub struct ErgoBlockProvider {
    pub client: Arc<ErgoClient>,
    pub processor: Arc<ErgoBlockProcessor>,
}

impl ErgoBlockProvider {
    pub fn new(ergo_config: &ErgoConfig, db_schema: DbSchema) -> Self {
        ErgoBlockProvider {
            client: Arc::new(ErgoClient {
                node_url: Url::from_str(&ergo_config.api_host).unwrap(),
                api_key: ergo_config.api_key.clone(),
            }),
            processor: Arc::new(ErgoBlockProcessor::new(ErgoIoProcessor::new(db_schema))),
        }
    }
}

#[async_trait]
impl BlockProvider for ErgoBlockProvider {
    type OutTx = EuTx;

    fn get_schema(&self) -> DbSchema {
        self.processor.io_processor.db_schema.clone()
    }

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block<Self::OutTx>, String> {
        let block = self.client.get_block_by_hash_sync(header.hash)?;
        self.processor.process_block(&block)
    }

    async fn get_chain_tip(&self) -> Result<BlockHeader, String> {
        self.client
            .get_best_block_async()
            .await
            .and_then(|b| self.processor.process_block(&b))
            .map(|b| b.header)
    }

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<EuTx>>, TxCount)> + Send + 'life0>> {
        let best_header = self.get_chain_tip().await.unwrap();
        let last_height = last_header.map_or(1, |h| h.height.0);
        info!("Indexing from {} to {}", last_height, best_header);
        let heights = last_height..=best_header.height.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn(async move {
                    client
                        .get_block_by_height_async(height.into())
                        .await
                        .unwrap()
                })
            })
            .buffered(num_cpus::get() / 2)
            .map(|res| match res {
                Ok(block) => {
                    let processor = Arc::clone(&self.processor);
                    tokio::task::spawn_blocking(move || processor.process_block(&block).unwrap())
                }
                Err(e) => panic!("Error: {:?}", e),
            })
            .buffered(num_cpus::get() / 2)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.weight)
            .boxed()
    }
}
