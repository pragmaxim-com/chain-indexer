use futures::StreamExt;

use crate::{
    api::{BlockMonitor, BlockProvider},
    indexer::Indexer,
    info,
    rocks_db_batch::CustomFamilies,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct ChainSyncer<'db, CF: CustomFamilies<'db>, OutTx: Send + 'static> {
    pub is_shutdown: Arc<AtomicBool>,
    pub block_source: Arc<dyn BlockProvider<OutTx = OutTx>>,
    pub monitor: Arc<dyn BlockMonitor<OutTx>>,
    pub indexer: Arc<Indexer<'db, CF, OutTx>>,
}

impl<'db, CF: CustomFamilies<'db>, OutTx: Send + 'static> ChainSyncer<'db, CF, OutTx> {
    pub fn new(
        block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
        monitor: Arc<dyn BlockMonitor<OutTx>>,
        indexer: Arc<Indexer<'db, CF, OutTx>>,
    ) -> Self {
        ChainSyncer {
            is_shutdown: Arc::new(AtomicBool::new(false)),
            block_source: block_provider,
            monitor,
            indexer,
        }
    }

    pub async fn sync(&self, min_batch_size: usize) {
        let is_chain_tip = false; // TODO check for ChainTip presence to start chainlinking
        self.block_source
            .stream(self.indexer.get_last_header(), min_batch_size)
            .await
            .map(|(block_batch, tx_count)| {
                let chain_link = block_batch.last().is_some_and(|_| is_chain_tip);
                self.monitor.monitor(&block_batch, &tx_count);
                self.indexer
                    .persist_blocks(block_batch, chain_link)
                    .unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e))
            })
            .count()
            .await;
    }

    pub fn flush_and_shutdown(&self) {
        if !self.is_shutdown.load(Ordering::SeqCst) {
            info!("Acquiring db lock for flushing closing...");
            self.indexer
                .storage
                .write()
                .unwrap()
                .db
                .flush()
                .expect("Failed to flush RocksDB");
            self.is_shutdown.store(true, Ordering::SeqCst);
            info!("RocksDB successfully flushed and closed.");
        }
    }
}

impl<'db, CF: CustomFamilies<'db>, OutTx: Send + 'static> Drop for ChainSyncer<'db, CF, OutTx> {
    fn drop(&mut self) {
        info!("Dropping indexer");
        self.flush_and_shutdown();
    }
}
