use crate::api::BlockHeaderLike;
use crate::api::{BlockLike, BlockPersistence};
use crate::api::{BlockProvider, ChainSyncError};
use crate::block_monitor::BlockMonitor;
use crate::info;
use futures::StreamExt;
use std::{
    rc::Rc,
    sync::Arc,
};

pub struct ChainSyncer<B: BlockLike> {
    pub block_provider: Arc<dyn BlockProvider<B>>,
    pub block_persistence: Arc<dyn BlockPersistence<B>>,
    pub monitor: Rc<BlockMonitor>,
}

impl<B: BlockLike> ChainSyncer<B> {
    pub fn new(
        block_provider: Arc<dyn BlockProvider<B>>,
        block_persistence: Arc<dyn BlockPersistence<B>>,
    ) -> Self {
        Self {
            block_provider,
            block_persistence,
            monitor: Rc::new(BlockMonitor::new(1000)),
        }
    }

    pub async fn sync(&self, min_batch_size: usize, fetching_par: usize, processing_par: usize) {
        let chain_tip_header = self.block_provider.get_chain_tip().await.expect("Failed to get chain tip header");
        let last_header = self.block_persistence.get_last_header().expect("Failed to get last header");

        self.block_provider
            .stream(
                chain_tip_header.clone(),
                last_header,
                min_batch_size,
                fetching_par,
                processing_par,
            )
            .await
            .for_each(|(block_batch, batch_weight)| {
                let chain_tip_header = chain_tip_header.clone();
                let this = self;

                async move {
                    let chain_link = block_batch.last().is_some_and(|curr_block| {
                        curr_block.header().height() + 100 > chain_tip_header.height()
                    });

                    let last_block = block_batch.last().expect("Block batch should not be empty");

                    let height = last_block.header().height();
                    let timestamp = last_block.header().timestamp();

                    this.monitor
                        .monitor(height, timestamp, block_batch.len(), &batch_weight);

                    this.persist_blocks(block_batch, chain_link)
                        .unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e.error));
                }
            })
            .await;
    }
    fn chain_link(
        &self,
        block: Arc<B>,
        winning_fork: &mut Vec<Arc<B>>,
    ) -> Result<Vec<Arc<B>>, ChainSyncError> {
        let header = block.header();
        let prev_headers = self.block_persistence.get_header_by_hash(header.prev_hash())?;

        if header.height() == 1 {
            winning_fork.insert(0, Arc::clone(&block));
            Ok(winning_fork.clone())
        } else if prev_headers
            .first()
            .is_some_and(|ph| ph.height() == header.height() - 1)
        {
            winning_fork.insert(0, Arc::clone(&block));
            Ok(winning_fork.clone())
        } else if prev_headers.is_empty() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                header.height(),
                hex::encode(header.hash()),
                hex::encode(header.prev_hash()),
            );

            let downloaded_prev_block =
                Arc::new(self.block_provider.get_processed_block(header.clone())?);

            winning_fork.insert(0, Arc::clone(&block));
            self.chain_link(downloaded_prev_block, winning_fork)
        } else {
            panic!(
                "Unexpected condition in chain_link: multiple parent candidates found for {}@{}",
                header.height(),
                hex::encode(header.hash())
            );
        }
    }

    pub fn persist_blocks(&self, blocks: Vec<B>, chain_link: bool) -> Result<(), ChainSyncError> {
        blocks
            .into_iter()
            .map(|block| {
                if chain_link {
                    self.chain_link(Arc::new(block), &mut vec![]).unwrap()
                } else {
                    vec![Arc::new(block)]
                }
            })
            .for_each(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    self.block_persistence
                        .store_blocks(&linked_blocks)
                        .expect("Failed to store blocks");
                }
                _ => {
                    self.block_persistence
                        .update_blocks(&linked_blocks)
                        .expect("Failed to update blocks");
                }
            });

        Ok(())
    }
}
