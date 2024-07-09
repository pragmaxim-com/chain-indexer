use crate::api::{Block, BlockMonitor, ChainLinker, Indexer};
use futures::stream::StreamExt;
use min_batch::ext::MinBatchExt;
use std::sync::Arc;

pub struct ChainSyncer<InBlock: Block + Send, OutBlock: Block + Send> {
    pub chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    pub monitor: Arc<dyn BlockMonitor<OutBlock> + Send + Sync>,
    pub indexers: Arc<dyn Indexer<OutBlock = OutBlock> + Send + Sync>,
}

impl<InBlock: Block + Send + Sync + 'static, OutBlock: Block + Send + Sync + Clone + 'static>
    ChainSyncer<InBlock, OutBlock>
{
    pub fn new(
        chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
        monitor: Arc<dyn BlockMonitor<OutBlock> + Send + Sync>,
        indexers: Arc<dyn Indexer<OutBlock = OutBlock> + Send + Sync>,
    ) -> Self {
        ChainSyncer {
            chain_linker,
            monitor,
            indexers,
        }
    }

    pub async fn sync(&self, min_batch_size: usize) -> () {
        let best_height = self.chain_linker.get_best_block().unwrap().height();
        let last_height = self.indexers.get_last_height() + 1;
        // let check_forks: bool = best_height - last_height < 1000;
        let heights = last_height..=best_height;
        tokio_stream::iter(heights)
            .map(|height| {
                let chain_linker = Arc::clone(&self.chain_linker);
                tokio::task::spawn_blocking(move || {
                    chain_linker.get_block_by_height(height).unwrap()
                })
            })
            .buffered(64)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.tx_count())
            .map(|(blocks, tx_count)| {
                let chain_linker = Arc::clone(&self.chain_linker);
                tokio::task::spawn_blocking(move || chain_linker.process_batch(&blocks, tx_count))
            })
            .buffered(256)
            .map(|res| match res {
                Ok((block_batch, tx_count)) => {
                    let _ = &self.monitor.monitor(&block_batch, tx_count);
                    block_batch
                }
                Err(e) => panic!("Error: {:?}", e),
            })
            .map(|blocks| {
                let indexers = Arc::clone(&self.indexers);
                indexers.consume(&blocks)
            })
            .count()
            .await;
    }
}
