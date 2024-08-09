use rocksdb::MultiThreaded;
use rocksdb::OptimisticTransactionDB;
use rocksdb::OptimisticTransactionOptions;
use rocksdb::WriteOptions;

use crate::api::BlockProvider;
use crate::api::Storage;
use crate::block_service::BlockService;
use crate::codec_block;
use crate::info;
use crate::model::Block;
use crate::model::BlockHeader;
use crate::model::Transaction;
use crate::rocks_db_batch::CustomFamilies;
use crate::rocks_db_batch::Families;
use std::rc::Rc;

use std::sync::Arc;
use std::sync::RwLock;

pub const LAST_HEADER_KEY: &[u8] = b"last_header";

pub struct Indexer<'db, CF: CustomFamilies<'db>, OutTx: Transaction + Send> {
    pub storage: Arc<RwLock<Storage>>,
    families: Arc<Families<'db, CF>>,
    service: Arc<BlockService<'db, OutTx, CF>>,
    block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
    disable_wal: bool,
}

impl<'db, CF: CustomFamilies<'db>, OutTx: Transaction + Send> Indexer<'db, CF, OutTx> {
    pub fn new(
        storage: Arc<RwLock<Storage>>,
        families: Arc<Families<'db, CF>>,
        service: Arc<BlockService<'db, OutTx, CF>>,
        block_provider: Arc<dyn BlockProvider<OutTx = OutTx>>,
        disable_wal: bool,
    ) -> Self {
        Indexer {
            storage,
            families,
            service,
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
            &self.families.shared.meta_cf,
            LAST_HEADER_KEY,
            codec_block::block_header_to_bytes(header),
        )?;
        Ok(())
    }

    pub fn get_last_header(&self) -> Option<BlockHeader> {
        let storage = self.storage.read().unwrap();
        storage
            .db
            .get_cf(&self.families.shared.meta_cf, LAST_HEADER_KEY)
            .unwrap()
            .map(|header_bytes| codec_block::bytes_to_block_header(&header_bytes))
    }

    fn chain_link(
        &self,
        block: Rc<Block<OutTx>>, // Use Rc to manage ownership and avoid lifetimes issues
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        winning_fork: &mut Vec<Rc<Block<OutTx>>>, // Use Rc for the vector as well
        families: &Families<'db, CF>,
    ) -> Result<Vec<Rc<Block<OutTx>>>, String> {
        let prev_header: Option<BlockHeader> = self
            .service
            .get_block_header_by_hash(&block.header.prev_hash, db_tx, families)
            .unwrap();

        if block.header.height.0 == 1 {
            winning_fork.insert(0, Rc::clone(&block)); // Clone the Rc, not the block
            Ok(winning_fork.clone())
        } else if prev_header
            .as_ref()
            .is_some_and(|ph| ph.height.0 == block.header.height.0 - 1)
        {
            winning_fork.insert(0, Rc::clone(&block));
            Ok(winning_fork.clone())
        } else if prev_header.is_none() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                block.header.height, block.header.hash, block.header.prev_hash,
            );
            let downloaded_prev_block = Rc::new(
                self.block_provider
                    .get_processed_block(block.header.clone())?,
            );

            winning_fork.insert(0, Rc::clone(&block));
            self.chain_link(downloaded_prev_block, db_tx, winning_fork, families)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub fn persist_blocks(
        &self,
        blocks: Vec<Block<OutTx>>,
        chain_link: bool,
    ) -> Result<(), String> {
        let mut write_options = WriteOptions::default();
        write_options.disable_wal(self.disable_wal);

        let storage = self.storage.write().unwrap();

        let db_tx: rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>> = storage
            .db
            .transaction_opt(&write_options, &OptimisticTransactionOptions::default());

        let mut batch = db_tx.get_writebatch();

        let last_block_header = blocks
            .into_iter()
            .map(|block| {
                if chain_link {
                    self.chain_link(Rc::new(block), &db_tx, &mut vec![], &self.families)
                        .unwrap()
                } else {
                    vec![Rc::new(block)]
                }
            })
            .map(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    self.service
                        .persist_blocks(linked_blocks, &db_tx, &mut batch, &self.families)
                        .unwrap();
                    last_header
                }
                _ => {
                    let last_header = linked_blocks.last().unwrap().header.clone();
                    self.service
                        .update_blocks(linked_blocks, &db_tx, &mut batch, &self.families)
                        .unwrap();
                    last_header
                }
            })
            .last();

        // persist last height to db_tx and commit
        if let Some(header) = last_block_header {
            self.persist_last_block(&header, &db_tx)
                .map_err(|e| e.into_string())?;

            db_tx.commit().map_err(|e| {
                eprintln!("Failed to commit transaction: {}", e);
                e.into_string()
            })?;
        }
        Ok(())
    }
}
