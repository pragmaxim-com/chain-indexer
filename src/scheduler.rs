use crate::api::{BlockLike, BlockPersistence, BlockProvider};
use crate::block_monitor::BlockMonitor;
use crate::settings::IndexerSettings;
use crate::syncer::ChainSyncer;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub struct Scheduler<B: BlockLike> {
    pub syncer: ChainSyncer<B>,
}

impl<B: BlockLike> Scheduler<B> {
    pub fn new(
        block_provider: Arc<dyn BlockProvider<B>>,
        block_persistence: Arc<dyn BlockPersistence<B>>,
    ) -> Self {
        let syncer = ChainSyncer {
            block_provider,
            block_persistence,
            monitor: Rc::new(BlockMonitor::new(1000)),
        };
        Scheduler { syncer }
    }

    pub async fn schedule(&self, indexer_conf: &IndexerSettings) {
        async {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                self.syncer
                    .sync(
                        indexer_conf.min_batch_size,
                        indexer_conf.fetching_parallelism.to_numeric(),
                        indexer_conf.processing_parallelism.to_numeric(),
                    )
                    .await;
                interval.tick().await;
            }
        }
        .await;
    }
}
