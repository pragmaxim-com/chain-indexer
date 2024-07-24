use crate::{
    api::{BlockMonitor, BlockProvider},
    indexer::Indexer,
    info,
    model::Transaction,
};
use futures::stream::StreamExt;

use min_batch::ext::MinBatchExt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct ChainSyncer<InTx: Send + 'static, OutTx: Transaction + Send + 'static> {
    pub is_shutdown: Arc<AtomicBool>,
    pub block_provider: Arc<dyn BlockProvider<InTx = InTx, OutTx = OutTx> + Send + Sync>,
    pub monitor: Arc<dyn BlockMonitor<OutTx>>,
    pub indexer: Arc<Indexer<InTx, OutTx>>,
}

impl<InTx: Send + 'static, OutTx: Transaction + Send + 'static> ChainSyncer<InTx, OutTx> {
    pub fn new(
        block_provider: Arc<dyn BlockProvider<InTx = InTx, OutTx = OutTx> + Send + Sync>,
        monitor: Arc<dyn BlockMonitor<OutTx>>,
        indexer: Arc<Indexer<InTx, OutTx>>,
    ) -> Self {
        ChainSyncer {
            is_shutdown: Arc::new(AtomicBool::new(false)),
            block_provider,
            monitor,
            indexer,
        }
    }

    pub async fn sync(&self, min_batch_size: usize) {
        let best_height = self.block_provider.get_best_block().unwrap().header.height;
        let last_height = self.indexer.get_last_height().0 + 1;
        // let check_forks: bool = best_height - last_height < 1000;
        info!("Initiating index from {} to {}", last_height, best_height);
        let heights = last_height..=best_height.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let block_provider = Arc::clone(&self.block_provider);
                tokio::task::spawn_blocking(move || {
                    block_provider.get_block_by_height(height.into()).unwrap()
                })
            })
            .buffered(num_cpus::get())
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.txs.len())
            .map(|(blocks, tx_count)| {
                let block_provider = Arc::clone(&self.block_provider);
                tokio::task::spawn_blocking(move || block_provider.process_batch(&blocks, tx_count))
            })
            .buffered(num_cpus::get())
            .map(|res| match res {
                Ok((block_batch, tx_count)) => {
                    let chain_link = block_batch
                        .last()
                        .is_some_and(|b| best_height.0 < b.header.height.0 + 1000);
                    self.monitor.monitor(&block_batch, &tx_count);
                    self.indexer
                        .persist_blocks(block_batch, chain_link)
                        .unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e))
                }
                Err(e) => panic!("Unable to process blocks: {:?}", e),
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

impl<InTx: Send + 'static, OutTx: Transaction + Send + 'static> Drop for ChainSyncer<InTx, OutTx> {
    fn drop(&mut self) {
        info!("Dropping indexer");
        self.flush_and_shutdown();
    }
}
