use rocksdb::ColumnFamily;
use rocksdb::WriteBatchWithTransaction;

use crate::api::ChainLinker;
use crate::block_service::BlockService;
use crate::codec_block;
use crate::eutxo::eutxo_model::*;
use crate::info;
use crate::model::Transaction;
use crate::model::{Block, BlockHeight, DbIndexName};
use crate::storage::Storage;
use crate::storage::Tx;
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

    fn persist_last_height(
        &self,
        height: BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error> {
        let batch = batch.borrow_mut();
        batch.db_tx.put_cf(
            batch.meta_cf,
            LAST_ADDRESS_HEIGHT_KEY,
            codec_block::block_height_to_bytes(&height),
        )
    }

    pub fn get_last_height(&self) -> BlockHeight {
        let db = self.db_holder.db.read().unwrap();
        db.get_cf(db.cf_handle(META_CF).unwrap(), LAST_ADDRESS_HEIGHT_KEY)
            .unwrap()
            .map_or(0.into(), |height| {
                codec_block::vector_to_block_height(&height)
            })
    }

    fn chain_link(
        &self,
        block: &Block<OutTx>,
        batch: &RefCell<RocksDbBatch>,
        winning_fork: &mut Vec<Block<OutTx>>,
    ) -> Result<Vec<Block<OutTx>>, String> {
        let header = block.header;
        let prev_height: Option<BlockHeight> = self
            .service
            .get_block_height_by_hash(&header.parent_hash, batch)
            .unwrap();
        if header.height.0 == 1 {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else if prev_height.is_some_and(|ph| ph.0 == header.height.0 - 1) {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else if prev_height.is_none() {
            info!(
                "Fork detected at {}@{}, downloading parent {}",
                header.height, header.hash, header.parent_hash,
            );
            let downloaded_prev_block = self
                .chain_linker
                .get_processed_block_by_hash(header.parent_hash)?;

            winning_fork.insert(0, block.clone());
            self.chain_link(&downloaded_prev_block, batch, winning_fork)
        } else {
            panic!("Unexpected condition") // todo pretty print blocks
        }
    }

    pub(crate) fn persist_blocks(&self, blocks: &Vec<Block<OutTx>>) -> Result<(), String> {
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
            index_cf_by_name: self
                .db_holder
                .utxo_indexes
                .iter()
                .map(|index_name| (index_name.clone(), db.cf_handle(&index_name).unwrap()))
                .collect::<Vec<(DbIndexName, &ColumnFamily)>>(),
        });

        blocks
            .into_iter()
            .map(|block| self.chain_link(block, &batch, &mut vec![]).unwrap())
            .for_each(|linked_blocks| match linked_blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => linked_blocks
                    .into_iter()
                    .for_each(|block| self.service.persist_block(block, &batch).unwrap()),
                _ => {
                    self.service.update_blocks(&linked_blocks, &batch).unwrap();
                }
            });

        // persist last height to db_tx if Some
        if let Some(block) = blocks.last() {
            self.persist_last_height(block.header.height, &batch)?;
        }
        db_tx.commit().map_err(|e| e.into_string())?;
        Ok(())
    }
}

pub struct RocksDbBatch<'db> {
    pub(crate) db_tx: &'db Tx<'db>,
    pub(crate) batch: &'db mut WriteBatchWithTransaction<true>,
    pub(crate) block_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) block_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) tx_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) tx_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) utxo_value_by_pk_cf: &'db ColumnFamily,
    pub(crate) utxo_pk_by_input_pk_cf: &'db ColumnFamily,
    pub(crate) meta_cf: &'db ColumnFamily,
    pub(crate) index_cf_by_name: Vec<(DbIndexName, &'db ColumnFamily)>,
}
