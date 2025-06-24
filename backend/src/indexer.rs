
use crate::api::BlockProvider;
use crate::api::ServiceError;
use crate::info;
use std::sync::Arc;
use redb::{Database, ReadTransaction, WriteTransaction};
use redbit::AppError;
use crate::eutxo::eutxo_model::{Block, BlockHeader};

pub const LAST_HEADER_KEY: &[u8] = b"last_header";

pub struct Indexer {
    pub db: Arc<Database>,
    block_provider: Arc<dyn BlockProvider>,
}

impl Indexer {
    pub fn new(db: Arc<Database>, block_provider: Arc<dyn BlockProvider>) -> Self {
        Indexer {
            db,
            block_provider,
        }
    }

    fn persist_last_block(
        &self,
        header: &BlockHeader,
        tx: &WriteTransaction,
    ) -> Result<(), AppError> {
        BlockHeader::store(tx, header)
    }

    pub fn get_last_header(&self, tx: &ReadTransaction) -> Option<BlockHeader> {
        BlockHeader::last(tx).unwrap()
    }

    fn chain_link(
        &self,
        block: Arc<Block>, // Use Arc to manage ownership and avoid lifetimes issues
        winning_fork: &mut Vec<Arc<Block>>, // Use Rc for the vector as well
        tx: &ReadTransaction
    ) -> Result<Vec<Arc<Block>>, ServiceError> {
        let prev_header = BlockHeader::get_by_hash(tx, &block.header.prev_hash).map_err(|e| {
            ServiceError::new(&format!(
                "Failed to get previous block header: {}",
                e
            ))
        })?;

        if block.header.id.0 == 1 {
            winning_fork.insert(0, Arc::clone(&block)); // Clone the Rc, not the block
            Ok(winning_fork.clone())
        } else if prev_header.first()
            .is_some_and(|ph| ph.id.0 == block.header.id.0 - 1)
        {
            winning_fork.insert(0, Arc::clone(&block));
            Ok(winning_fork.clone())
        } else if prev_header.first().is_none() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                block.header.id, block.header.hash, block.header.prev_hash,
            );
            let read_tx = self.db.begin_read().map_err(|e| {
                ServiceError::new(&format!("Failed to begin read transaction: {}", e))
            })?;
            let downloaded_prev_block = Arc::new(
                self.block_provider
                    .get_processed_block(block.header.clone(), &read_tx)?,
            );

            winning_fork.insert(0, Arc::clone(&block));
            self.chain_link(downloaded_prev_block, winning_fork, tx)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub fn persist_blocks(
        &self,
        blocks: Vec<Block>,
        chain_link: bool,
    ) -> Result<(), ServiceError> {
        let write_tx = self.db.begin_write().map_err(|e| {
            ServiceError::new(&format!("Failed to begin write transaction: {}", e))
        })?;
        let read_tx = self.db.begin_read().unwrap();
        let last_block_header = blocks
            .into_iter()
            .map(|block| {
                if chain_link {
                    self.chain_link(Arc::new(block), &mut vec![], &read_tx).unwrap()
                } else {
                    vec![Arc::new(block)]
                }
            })
            .map(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    for linked_block in &linked_blocks {
                        Block::store(&write_tx, linked_block).unwrap();
                    }
                    last_header
                }
                _ => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    for linked_block in &linked_blocks {
                        // TODO remove block
                        Block::store(&write_tx, linked_block).unwrap();
                    }
                    last_header
                }
            })
            .last();

        // persist last height to db_tx and commit
        if let Some(header) = last_block_header {
            self.persist_last_block(&header, &write_tx)
                .map_err(|e| ServiceError::new(&e.to_string()))?;
        }
        write_tx.commit().map_err(|e| {
            ServiceError::new(&format!("Failed to commit write transaction: {}", e))
        })?;
        Ok(())
    }
}
