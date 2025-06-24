use futures::StreamExt;

use crate::{
    api::{BlockMonitor, BlockProvider},
    indexer::Indexer,
};
use std::{
    rc::Rc,
    sync::{
        atomic::AtomicBool,
        Arc,
    },
};

pub struct ChainSyncer {
    pub is_shutdown: Arc<AtomicBool>,
    pub block_provider: Arc<dyn BlockProvider>,
    pub monitor: Rc<dyn BlockMonitor>,
    pub indexer: Indexer,
}

impl ChainSyncer {
    pub fn new(
        block_provider: Arc<dyn BlockProvider>,
        monitor: Rc<dyn BlockMonitor>,
        indexer: Indexer,
    ) -> Self {
        ChainSyncer {
            is_shutdown: Arc::new(AtomicBool::new(false)),
            block_provider,
            monitor,
            indexer,
        }
    }

    pub async fn sync(&self, min_batch_size: usize, fetching_par: usize, processing_par: usize) {
        let read_tx = self.indexer.db.begin_read().unwrap();
        let chain_tip_header = self.block_provider.get_chain_tip(&read_tx).await.unwrap();
        let tx = self.indexer.db.begin_read().unwrap();
        self.block_provider
            .stream(
                self.indexer.get_last_header(&tx),
                min_batch_size,
                fetching_par,
                processing_par,
            )
            .await
            .for_each(|(block_batch, batch_weight)| async move {
                let chain_link = block_batch.last().is_some_and(|curr_block| {
                    curr_block.header.id.0 + 100 > chain_tip_header.id.0
                });
                self.monitor.monitor(&block_batch, &batch_weight);
                self.indexer
                    .persist_blocks(block_batch, chain_link)
                    .unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e.error))
            })
            .await;
    }

}
