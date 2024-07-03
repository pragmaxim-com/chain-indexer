use crate::api::{BlockProcessor, BlockchainClient, Indexers};
use crate::info;
use crate::monitor::BlockMonitor;
use broadcast_sink::StreamBroadcastSinkExt;
use futures::stream::StreamExt;
use min_batch::MinBatchExt;
use std::sync::Arc;

pub struct ChainSyncer<InBlock: Send, OutBlock: Send> {
    pub client: Arc<dyn BlockchainClient<Block = InBlock> + Send + Sync>,
    pub monitor: Arc<dyn BlockMonitor<InBlock>>,
    pub processor: Arc<dyn BlockProcessor<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    pub indexers: Arc<dyn Indexers<OutBlock = OutBlock>>,
}

impl<InBlock: Send + 'static, OutBlock: Send + Sync + Clone + 'static>
    ChainSyncer<InBlock, OutBlock>
{
    pub fn new(
        client: Arc<dyn BlockchainClient<Block = InBlock> + Send + Sync>,
        monitor: Arc<dyn BlockMonitor<InBlock>>,
        processor: Arc<dyn BlockProcessor<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
        indexers: Arc<dyn Indexers<OutBlock = OutBlock> + Send + Sync>,
    ) -> Self {
        ChainSyncer {
            client,
            monitor,
            processor,
            indexers,
        }
    }

    pub async fn sync(&self, end_height: u32, min_batch_size: usize) -> () {
        let last_height = self.indexers.get_last_height() + 1;
        info!("Indexing from {} to {}", last_height, end_height);
        let heights = last_height..=end_height;
        tokio_stream::iter(heights)
            .map(|height| {
                let rpc_client = Arc::clone(&self.client);
                tokio::task::spawn_blocking(move || rpc_client.get_block(height).unwrap())
            })
            .buffered(10000)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch(min_batch_size, |(_, _, tx_count, _)| *tx_count)
            .map(|blocks| {
                let _ = &self.monitor.monitor(&blocks);
                blocks
            })
            .map(|blocks| {
                let processor = Arc::clone(&self.processor);
                tokio::task::spawn_blocking(move || processor.process(&blocks))
            })
            .buffered(512)
            .map(|res| match res {
                Ok(blocks) => blocks,
                Err(e) => panic!("Error: {:?}", e),
            })
            .broadcast(min_batch_size, self.indexers.get_indexers())
            .await;
    }
}
