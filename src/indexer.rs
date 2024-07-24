use rocksdb::ColumnFamily;

use crate::api::BlockProvider;
use crate::block_service::BlockService;
use crate::codec_block;
use crate::eutxo::eutxo_model::*;
use crate::info;
use crate::model::BlockHeader;
use crate::model::Transaction;
use crate::model::{Block, BlockHeight};
use crate::rocks_db_batch::RocksDbBatch;
use crate::storage::Storage;
use std::rc::Rc;

use std::cell::RefCell;
use std::sync::Arc;

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

pub struct Indexer<InTx: Send, OutTx: Transaction + Send> {
    pub db_holder: Arc<Storage>,
    service: Arc<BlockService<OutTx>>,
    block_provider: Arc<dyn BlockProvider<InTx = InTx, OutTx = OutTx> + Send + Sync>,
}

impl<InTx: Send, OutTx: Transaction + Send> Indexer<InTx, OutTx> {
    pub fn new(
        db: Arc<Storage>,
        service: Arc<BlockService<OutTx>>,
        block_provider: Arc<dyn BlockProvider<InTx = InTx, OutTx = OutTx> + Send + Sync>,
    ) -> Self {
        Indexer {
            db_holder: db,
            service,
            block_provider,
        }
    }

    fn persist_last_height(
        &self,
        height: BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error> {
        let batch = batch.borrow_mut();
        let db_tx = batch.db_tx;
        db_tx.put_cf(
            batch.meta_cf,
            LAST_ADDRESS_HEIGHT_KEY,
            codec_block::block_height_to_bytes(&height),
        )?;
        Ok(())
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
        block: Rc<Block<OutTx>>, // Use Rc to manage ownership and avoid lifetimes issues
        batch: &RefCell<RocksDbBatch>,
        winning_fork: &mut Vec<Rc<Block<OutTx>>>, // Use Rc for the vector as well
    ) -> Result<Vec<Rc<Block<OutTx>>>, String> {
        let prev_header: Option<BlockHeader> = self
            .service
            .get_block_header_by_hash(&block.header.prev_hash, batch)
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
                    .get_processed_block_by_hash(block.header.prev_hash)?,
            );

            winning_fork.insert(0, Rc::clone(&block));
            self.chain_link(downloaded_prev_block, batch, winning_fork)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub fn persist_blocks(
        &self,
        blocks: Vec<Block<OutTx>>,
        chain_link: bool,
    ) -> Result<(), String> {
        let db = self.db_holder.db.write().unwrap();
        let db_tx = db.transaction();
        let mut binding = db_tx.get_writebatch();
        let batch = RefCell::new(RocksDbBatch {
            db_tx: &db_tx,
            batch: &mut binding,
            block_hash_by_pk_cf: db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
            block_pk_by_hash_cf: db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
            tx_hash_by_pk_cf: db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
            tx_pk_by_hash_cf: db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
            utxo_value_by_pk_cf: db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
            utxo_pk_by_input_pk_cf: db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
            meta_cf: db.cf_handle(META_CF).unwrap(),
            utxo_birth_pk_with_utxo_pk_cf: self
                .db_holder
                .db_index_manager
                .utxo_birth_pk_relations
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            utxo_birth_pk_by_index_cf: self
                .db_holder
                .db_index_manager
                .utxo_birth_pk_by_index
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            index_by_utxo_birth_pk_cf: self
                .db_holder
                .db_index_manager
                .index_by_utxo_birth_pk
                .iter()
                .map(|cf| db.cf_handle(&cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            assets_by_utxo_pk_cf: db.cf_handle(ASSETS_BY_UTXO_PK_CF).unwrap(),
            asset_id_by_asset_birth_pk_cf: db.cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF).unwrap(),
            asset_birth_pk_by_asset_id_cf: db.cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF).unwrap(),
            asset_birth_pk_with_asset_pk_cf: db.cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF).unwrap(),
        });

        let last_block_height = blocks
            .into_iter()
            .map(|block| {
                if chain_link {
                    self.chain_link(Rc::new(block), &batch, &mut vec![])
                        .unwrap()
                } else {
                    vec![Rc::new(block)]
                }
            })
            .map(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    let last_height = linked_blocks.last().unwrap().header.height;
                    linked_blocks
                        .into_iter()
                        .for_each(|block| self.service.persist_block(block, &batch).unwrap());
                    last_height
                }
                _ => {
                    let last_height = linked_blocks.last().unwrap().header.height;
                    self.service.update_blocks(linked_blocks, &batch).unwrap();
                    last_height
                }
            })
            .last();

        // persist last height to db_tx and commit
        if let Some(height) = last_block_height {
            self.persist_last_height(height, &batch)?;
            db_tx.commit().map_err(|e| e.into_string())?;
        }
        Ok(())
    }
}
