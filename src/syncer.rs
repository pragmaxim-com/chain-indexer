use crate::{
    api::{Block, BlockMonitor, ChainLinker},
    indexer::Indexer,
    info,
};
use futures::stream::StreamExt;
use min_batch::ext::MinBatchExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct ChainSyncer<
    InBlock: Block + Send + Sync + 'static,
    OutBlock: Block + Send + Sync + Clone + 'static,
> {
    pub is_shutdown: Arc<AtomicBool>,
    pub chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    pub monitor: Arc<dyn BlockMonitor<OutBlock> + Send + Sync>,
    pub indexer: Arc<Indexer<InBlock, OutBlock>>,
}

impl<InBlock: Block + Send + Sync + 'static, OutBlock: Block + Send + Sync + Clone + 'static>
    ChainSyncer<InBlock, OutBlock>
{
    pub fn new(
        chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
        monitor: Arc<dyn BlockMonitor<OutBlock> + Send + Sync>,
        indexers: Arc<Indexer<InBlock, OutBlock>>,
    ) -> Self {
        ChainSyncer {
            is_shutdown: Arc::new(AtomicBool::new(false)),
            chain_linker,
            monitor,
            indexer: indexers,
        }
    }

    pub async fn sync(&self, min_batch_size: usize) {
        let best_height = self.chain_linker.get_best_block().unwrap().height();
        let last_height = self.indexer.get_last_height() + 1;
        // let check_forks: bool = best_height - last_height < 1000;
        info!("Initiating index from {} to {}", last_height, best_height);
        let heights = last_height..=best_height;

        let chain_linker_clone = Arc::clone(&self.chain_linker);
        let indexer_clone = Arc::clone(&self.indexer);
        let monitor_clone = Arc::clone(&self.monitor);
        tokio_stream::iter(heights)
            .map(|height| {
                let chain_linker = Arc::clone(&chain_linker_clone);
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
                let chain_linker = Arc::clone(&chain_linker_clone);
                tokio::task::spawn_blocking(move || chain_linker.process_batch(&blocks, tx_count))
            })
            .buffered(64)
            .map(|res| match res {
                Ok((block_batch, tx_count)) => {
                    let _ = monitor_clone.monitor(&block_batch, tx_count);
                    block_batch
                }
                Err(e) => panic!("Error: {:?}", e),
            })
            .map(|blocks| {
                let indexer = Arc::clone(&indexer_clone);
                indexer.persist_blocks(&blocks)
            })
            .count()
            .await;
    }

    pub fn flush_and_shutdown(&self) {
        if !self.is_shutdown.load(Ordering::SeqCst) {
            info!("Acquiring db lock for flushing closing...");
            self.indexer
                .db_holder
                .db
                .write()
                .unwrap()
                .flush()
                .expect("Failed to flush RocksDB");
            self.is_shutdown.store(true, Ordering::SeqCst);
            info!("RocksDB successfully flushed and closed.");
        }
    }
}

impl<InBlock: Block + Send + Sync + 'static, OutBlock: Block + Send + Sync + Clone + 'static> Drop
    for ChainSyncer<InBlock, OutBlock>
{
    fn drop(&mut self) {
        info!("Dropping indexer");
        self.flush_and_shutdown();
    }
}
