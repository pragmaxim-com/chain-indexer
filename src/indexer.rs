use crate::api::ChainLinker;
use crate::block_service::BlockService;
use crate::codec_block;
use crate::eutxo::eutxo_model::*;
use crate::info;
use crate::model::BlockHeader;
use crate::model::Transaction;
use crate::model::{Block, BlockHeight};
use crate::rocks_db_batch::RocksDbBatch;
use crate::storage::Storage;

use std::cell::RefCell;
use std::sync::Arc;

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

pub struct Indexer<InTx: Send + Clone, OutTx: Transaction + Send + Clone> {
    pub db_holder: Arc<Storage>,
    service: Arc<BlockService<OutTx>>,
    chain_linker: Arc<dyn ChainLinker<InTx = InTx, OutTx = OutTx> + Send + Sync>,
}

impl<InTx: Send + Clone, OutTx: Transaction + Send + Clone> Indexer<InTx, OutTx> {
    pub fn new(
        db: Arc<Storage>,
        service: Arc<BlockService<OutTx>>,
        chain_linker: Arc<dyn ChainLinker<InTx = InTx, OutTx = OutTx> + Send + Sync>,
    ) -> Self {
        Indexer {
            db_holder: db,
            service,
            chain_linker,
        }
    }

    fn persist_last_height_and_commit(
        &self,
        height: BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error> {
        let batch = batch.borrow_mut();
        batch.db_tx.put_cf(
            batch.meta_cf,
            LAST_ADDRESS_HEIGHT_KEY,
            codec_block::block_height_to_bytes(&height),
        )?;
        batch.db_tx.commit()
    }

    pub fn get_last_height(&self) -> BlockHeight {
        let db = self.db_holder.db.read().unwrap();
        db.get_cf(db.cf_handle(META_CF).unwrap(), LAST_ADDRESS_HEIGHT_KEY)
            .unwrap()
            .map_or(0.into(), |height| {
                codec_block::bytes_to_block_height(&height)
            })
    }

    fn chain_link(
        &self,
        block: &Block<OutTx>,
        batch: &RefCell<RocksDbBatch>,
        winning_fork: &mut Vec<Block<OutTx>>,
    ) -> Result<Vec<Block<OutTx>>, String> {
        let header = block.header;
        let prev_header: Option<BlockHeader> = self
            .service
            .get_block_header_by_hash(&header.prev_hash, batch)
            .unwrap();
        if header.height.0 == 1 {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else if prev_header.is_some_and(|ph| ph.height.0 == header.height.0 - 1) {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else if prev_header.is_none() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                header.height, header.hash, header.prev_hash,
            );
            let downloaded_prev_block = self
                .chain_linker
                .get_processed_block_by_hash(header.prev_hash)?;

            winning_fork.insert(0, block.clone());
            self.chain_link(&downloaded_prev_block, batch, winning_fork)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub(crate) fn persist_blocks(&self, blocks: &Vec<Block<OutTx>>) -> Result<(), String> {
        let batch = RefCell::new(RocksDbBatch::new(self.db_holder));

        blocks
            .iter()
            .map(|block| self.chain_link(block, &batch, &mut vec![]).unwrap())
            .for_each(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => linked_blocks
                    .into_iter()
                    .for_each(|block| self.service.persist_block(&block, &batch).unwrap()),
                _ => {
                    self.service.update_blocks(&linked_blocks, &batch).unwrap();
                }
            });

        // persist last height to db_tx if Some
        if let Some(block) = blocks.last() {
            self.persist_last_height_and_commit(block.header.height, &batch)?;
        }
        batch.borrow_mut().commit().map_err(|e| e.into_string())?;
        Ok(())
    }
}
