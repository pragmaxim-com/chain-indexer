use crate::{
    api::{BlockProcessor, BlockProvider},
    eutxo::{eutxo_model::EuTx, eutxo_schema::DbSchema},
    info,
    model::{Block, BlockHeader, TxCount},
    settings::BitcoinConfig,
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use min_batch::ext::MinBatchExt;
use std::{pin::Pin, sync::Arc};

use super::{
    btc_block_processor::BtcBlockProcessor, btc_client::BtcClient, btc_io_processor::BtcIoProcessor,
};

pub struct BtcBlockProvider {
    pub client: Arc<BtcClient>,
    pub processor: Arc<BtcBlockProcessor>,
}

impl BtcBlockProvider {
    pub fn new(bitcoin_config: &BitcoinConfig, db_schema: DbSchema) -> Self {
        BtcBlockProvider {
            client: Arc::new(BtcClient::new(bitcoin_config)),
            processor: Arc::new(BtcBlockProcessor::new(BtcIoProcessor::new(db_schema))),
        }
    }

    pub fn process_batch(
        &self,
        block_batch: &Vec<Block<bitcoin::Transaction>>,
        tx_count: TxCount,
    ) -> (Vec<Block<EuTx>>, TxCount) {
        self.processor.process_batch(block_batch, tx_count)
    }

    pub(crate) async fn get_best_block_header(&self) -> Result<BlockHeader, String> {
        self.client.get_best_block()
    }
}

#[async_trait]
impl BlockProvider for BtcBlockProvider {
    type OutTx = EuTx;

    fn get_schema(&self) -> DbSchema {
        self.processor.io_processor.db_schema.clone()
    }

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block<Self::OutTx>, String> {
        let block = self.client.get_block_by_hash(header.hash)?;
        let processed_block = self.processor.process_block(&block);
        Ok(processed_block)
    }

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<EuTx>>, TxCount)> + Send + 'life0>> {
        let best_header = self.get_best_block_header().await.unwrap();
        let last_height = last_header.map_or(0, |h| h.height.0);
        info!("Initiating index from {} to {}", last_height, best_header);
        let heights = last_height..=best_header.height.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn_blocking(move || {
                    client.get_block_by_height(height.into()).unwrap()
                })
            })
            .buffered(num_cpus::get())
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.txs.len())
            .map(|(blocks, tx_count)| {
                let processor = Arc::clone(&self.processor);
                tokio::task::spawn_blocking(move || processor.process_batch(&blocks, tx_count))
            })
            .buffered(num_cpus::get())
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .boxed()
    }
}
