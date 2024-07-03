use crate::api::{Block, BlockMonitor, BlockProcessor, BlockchainClient, Indexers};
use crate::info;
use broadcast_sink::StreamBroadcastSinkExt;
use futures::stream::StreamExt;
use min_batch::ext::MinBatchExt;
use std::sync::Arc;

pub struct ChainSyncer<InBlock: Block + Send, OutBlock: Block + Send> {
    pub client: Arc<dyn BlockchainClient<Block = InBlock> + Send + Sync>,
    pub processor: Arc<dyn BlockProcessor<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    pub monitor: Arc<dyn BlockMonitor<OutBlock>>,
    pub indexers: Arc<dyn Indexers<OutBlock = OutBlock>>,
}

impl<InBlock: Block + Send + 'static, OutBlock: Block + Send + Sync + Clone + 'static>
    ChainSyncer<InBlock, OutBlock>
{
    pub fn new(
        client: Arc<dyn BlockchainClient<Block = InBlock> + Send + Sync>,
        processor: Arc<dyn BlockProcessor<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
        monitor: Arc<dyn BlockMonitor<OutBlock>>,
        indexers: Arc<dyn Indexers<OutBlock = OutBlock> + Send + Sync>,
    ) -> Self {
        ChainSyncer {
            client,
            processor,
            monitor,
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
            .min_batch_with_weight(min_batch_size, |block| block.tx_count())
            .map(|(blocks, tx_count)| {
                let processor = Arc::clone(&self.processor);
                tokio::task::spawn_blocking(move || processor.process(&blocks, tx_count))
            })
            .buffered(512)
            .map(|res| match res {
                Ok((block_batch, tx_count)) => {
                    let _ = &self.monitor.monitor(&block_batch, tx_count);
                    block_batch
                }
                Err(e) => panic!("Error: {:?}", e),
            })
            .broadcast(min_batch_size, self.indexers.get_indexers())
            .await;
    }
}
