use futures::StreamExt;

use crate::{
    api::{BlockMonitor, BlockProvider},
    indexer::Indexer,
    info,
    rocks_db_batch::CustomFamilies,
};
use std::{
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct ChainSyncer<'db, CF: CustomFamilies<'db>, OutTx: Send + 'static> {
    pub is_shutdown: Arc<AtomicBool>,
    pub block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
    pub monitor: Rc<dyn BlockMonitor<OutTx>>,
    pub indexer: Indexer<'db, CF, OutTx>,
}

impl<'db, CF: CustomFamilies<'db>, OutTx: Send + 'static> ChainSyncer<'db, CF, OutTx> {
    pub fn new(
        block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
        monitor: Rc<dyn BlockMonitor<OutTx>>,
        indexer: Indexer<'db, CF, OutTx>,
    ) -> Self {
        ChainSyncer {
            is_shutdown: Arc::new(AtomicBool::new(false)),
            block_provider,
            monitor,
            indexer,
        }
    }

    pub async fn sync(&self, min_batch_size: usize) {
        let chain_tip_header = self.block_provider.get_chain_tip().await.unwrap();
        self.block_provider
            .stream(self.indexer.get_last_header(), min_batch_size)
            .await
            .for_each(|(block_batch, tx_count)| async move {
                let chain_link = block_batch.last().is_some_and(|curr_block| {
                    curr_block.header.height.0 + 100 > chain_tip_header.height.0
                });
                self.monitor.monitor(&block_batch, &tx_count);
                self.indexer
                    .persist_blocks(block_batch, chain_link)
                    .unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e))
            })
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
