use crate::api::BlockHeaderLike;
use crate::api::{BlockLike, BlockPersistence};
use crate::api::{BlockProvider, ChainSyncError};
use crate::info;
use crate::monitor::ProgressMonitor;
use futures::StreamExt;
use min_batch::ext::MinBatchExt;
use std::{rc::Rc, sync::Arc};

pub struct ChainSyncer<FB: Send, TB: BlockLike> {
    pub block_provider: Arc<dyn BlockProvider<FB, TB>>,
    pub block_persistence: Arc<dyn BlockPersistence<TB>>,
    pub monitor: Rc<ProgressMonitor>,
}

impl<FB: Send, TB: BlockLike> ChainSyncer<FB, TB> {
    pub fn new(block_provider: Arc<dyn BlockProvider<FB, TB>>, block_persistence: Arc<dyn BlockPersistence<TB>>) -> Self {
        Self { block_provider, block_persistence, monitor: Rc::new(ProgressMonitor::new(1000)) }
    }

    pub async fn sync(&self, min_batch_size: usize, _processing_par: usize) {
        let chain_tip_header = self.block_provider.get_chain_tip().await.expect("Failed to get chain tip header");
        let last_header = self.block_persistence.get_last_header().expect("Failed to get last header");

        self.block_provider
            .stream(chain_tip_header.clone(), last_header)
            .await
            .map(|block| self.block_provider.process_block(&block).expect("Failed to process block"))
            .min_batch_with_weight(min_batch_size, |block| block.weight() as usize)
            .for_each(|(block_batch, batch_weight)| {
                let chain_tip_header = chain_tip_header.clone();
                let this = self;

                async move {
                    let chain_link = block_batch.last().is_some_and(|curr_block| curr_block.header().height() + 100 > chain_tip_header.height());

                    let last_block = block_batch.last().expect("Block batch should not be empty");

                    let height = last_block.header().height();
                    let timestamp = last_block.header().timestamp();

                    this.monitor.log(height, timestamp, block_batch.len(), &batch_weight);

                    this.persist_blocks(block_batch, chain_link).unwrap_or_else(|e| panic!("Unable to persist blocks due to {}", e.error));
                }
            })
            .await;
    }

    fn chain_link(&self, block: TB) -> Result<Vec<TB>, ChainSyncError> {
        let header = block.header();
        let prev_headers = self.block_persistence.get_header_by_hash(header.prev_hash())?;

        // Base case: genesis
        if header.height() == 1 {
            return Ok(vec![block]);
        }

        // If the DB already has the direct predecessor, we can stop here
        if prev_headers.first().map(|ph| ph.height() == header.height() - 1).unwrap_or(false) {
            return Ok(vec![block]);
        }

        // Otherwise we need to fetch the parent and prepend it
        if prev_headers.is_empty() {
            info!("Fork detected at {}@{}, downloading parent {}", header.height(), hex::encode(header.hash()), hex::encode(header.prev_hash()),);

            // fetch parent
            let parent_header = header.clone();
            let parent_block = self.block_provider.get_processed_block(parent_header)?;
            // recurse to build the earlier part of the chain
            let mut chain = self.chain_link(parent_block)?;
            // now append our current block at the end
            chain.push(block);
            return Ok(chain);
        }

        // If we got here, there were multiple candidates in DB → panic or handle specially
        panic!("Unexpected condition in chain_link: multiple parent candidates for {}@{}", header.height(), hex::encode(header.hash()));
    }

    pub fn persist_blocks(&self, blocks: Vec<TB>, do_chain_link: bool) -> Result<(), ChainSyncError> {
        for block in blocks {
            // consume each block by value, build its chain
            let chain = if do_chain_link { self.chain_link(block)? } else { vec![block] };

            match chain.len() {
                0 => unreachable!("chain_link never returns empty Vec"),
                1 => self.block_persistence.store_blocks(chain)?,
                _ => self.block_persistence.update_blocks(chain)?,
            }
        }
        Ok(())
    }
}
