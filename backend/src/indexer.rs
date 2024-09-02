use model::Block;
use model::BlockHeader;
use rocksdb::MultiThreaded;
use rocksdb::OptimisticTransactionDB;
use rocksdb::OptimisticTransactionOptions;
use rocksdb::WriteOptions;

use crate::api::BlockProvider;
use crate::api::ServiceError;
use crate::block_write_service::BlockWriteService;
use crate::codec_block;
use crate::info;
use crate::persistence::Persistence;
use crate::rocks_db_batch::CustomFamilies;

use std::sync::Arc;

pub const LAST_HEADER_KEY: &[u8] = b"last_header";

pub struct Indexer<CF: CustomFamilies, OutTx: Send> {
    pub storage: Arc<Persistence<CF>>,
    block_write_service: Arc<BlockWriteService<OutTx, CF>>,
    block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
    disable_wal: bool,
}

impl<CF: CustomFamilies, OutTx: Send> Indexer<CF, OutTx> {
    pub fn new(
        storage: Arc<Persistence<CF>>,
        block_write_service: Arc<BlockWriteService<OutTx, CF>>,
        block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
        disable_wal: bool,
    ) -> Self {
        Indexer {
            storage,
            block_write_service,
            block_provider,
            disable_wal,
        }
    }

    fn persist_last_block(
        &self,
        header: &BlockHeader,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.put_cf(
            &self.storage.families.shared.meta_cf,
            LAST_HEADER_KEY,
            codec_block::block_header_to_bytes(header),
        )?;
        Ok(())
    }

    pub fn get_last_header(&self) -> Option<BlockHeader> {
        self.storage
            .db
            .get_cf(&self.storage.families.shared.meta_cf, LAST_HEADER_KEY)
            .unwrap()
            .map(|header_bytes| codec_block::bytes_to_block_header(&header_bytes))
    }

    fn chain_link(
        &self,
        block: Arc<Block<OutTx>>, // Use Arc to manage ownership and avoid lifetimes issues
        winning_fork: &mut Vec<Arc<Block<OutTx>>>, // Use Rc for the vector as well
    ) -> Result<Vec<Arc<Block<OutTx>>>, ServiceError> {
        let prev_header: Option<BlockHeader> = self
            .block_write_service
            .block_service
            .get_block_header_by_hash(&block.header.prev_hash)
            .unwrap();

        if block.header.height.0 == 1 {
            winning_fork.insert(0, Arc::clone(&block)); // Clone the Rc, not the block
            Ok(winning_fork.clone())
        } else if prev_header
            .as_ref()
            .is_some_and(|ph| ph.height.0 == block.header.height.0 - 1)
        {
            winning_fork.insert(0, Arc::clone(&block));
            Ok(winning_fork.clone())
        } else if prev_header.is_none() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                block.header.height, block.header.hash, block.header.prev_hash,
            );
            let downloaded_prev_block = Arc::new(
                self.block_provider
                    .get_processed_block(block.header.clone())?,
            );

            winning_fork.insert(0, Arc::clone(&block));
            self.chain_link(downloaded_prev_block, winning_fork)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub fn persist_blocks(
        &self,
        blocks: Vec<Block<OutTx>>,
        chain_link: bool,
    ) -> Result<(), ServiceError> {
        let mut write_options = WriteOptions::default();
        write_options.disable_wal(self.disable_wal);

        let db_tx: rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>> = self
            .storage
            .db
            .transaction_opt(&write_options, &OptimisticTransactionOptions::default());

        let mut batch = db_tx.get_writebatch();

        let last_block_header = blocks
            .into_iter()
            .map(|block| {
                if chain_link {
                    self.chain_link(Arc::new(block), &mut vec![]).unwrap()
                } else {
                    vec![Arc::new(block)]
                }
            })
            .map(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    self.block_write_service
                        .persist_blocks(linked_blocks, &db_tx, &mut batch, &self.storage.families)
                        .unwrap();
                    last_header
                }
                _ => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    self.block_write_service
                        .update_blocks(linked_blocks, &db_tx, &mut batch, &self.storage.families)
                        .unwrap();
                    last_header
                }
            })
            .last();

        // persist last height to db_tx and commit
        if let Some(header) = last_block_header {
            self.persist_last_block(&header, &db_tx)?;

            db_tx.commit()?;
        }
        Ok(())
    }
}
