use crate::api::Block;
use crate::api::BlockHeight;
use crate::api::ChainLinker;
use crate::api::Service;
use crate::rocksdb_wrapper::RocksDbWrapper;
use rocksdb::OptimisticTransactionDB;
use rocksdb::SingleThreaded;
use rocksdb::WriteBatchWithTransaction;
use std::sync::Arc;

pub struct Indexer<InBlock: Block + Send + Sync, OutBlock: Block + Send + Sync> {
    pub db: Arc<RocksDbWrapper>,
    service: Arc<dyn Service<OutBlock = OutBlock>>,
    chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
}

impl<InBlock: Block + Send + Sync, OutBlock: Block + Send + Sync> Indexer<InBlock, OutBlock> {
    pub fn new(
        db: Arc<RocksDbWrapper>,
        service: Arc<dyn Service<OutBlock = OutBlock>>,
        chain_linker: Arc<dyn ChainLinker<InBlock = InBlock, OutBlock = OutBlock> + Send + Sync>,
    ) -> Self {
        Indexer {
            db,
            service,
            chain_linker,
        }
    }
}

enum DbOp {
    Insert,
    Delete,
}

impl<InBlock: Block + Send + Sync, OutBlock: Block + Clone + Send + Sync>
    Indexer<InBlock, OutBlock>
{
    fn chain_link(
        &self,
        block: &OutBlock,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<SingleThreaded>>,
        winning_fork: &mut Vec<OutBlock>,
    ) -> Result<Vec<OutBlock>, String> {
        let prev_height: Option<BlockHeight> = self
            .service
            .get_block_height_by_hash(&block.prev_hash(), db_tx)
            .unwrap();
        if block.height() == 1 {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else if prev_height.is_some() {
            winning_fork.insert(0, block.clone());
            Ok(winning_fork.clone())
        } else {
            let prev_block = self
                .chain_linker
                .get_processed_block_by_hash(block.prev_hash())?;
            winning_fork.insert(0, block.clone());
            self.chain_link(&prev_block, db_tx, winning_fork)
        }
    }

    pub(crate) fn persist_blocks(&self, blocks: &Vec<OutBlock>) -> Result<(), String> {
        let db_tx = self.db.borrow_db().transaction();
        let mut batch: WriteBatchWithTransaction<true> = db_tx.get_writebatch();

        let mut tx_pk_by_tx_hash_lru_cache = self
            .service
            .get_tx_pk_by_tx_hash_lru_cache()
            .lock()
            .map_err(|e| e.to_string())?;

        blocks
            .iter()
            .map(|block| self.chain_link(block, &db_tx, &mut vec![]).unwrap())
            .for_each(|blocks| match blocks.len() {
                0 => panic!("Blocks vector is empty"),
                1 => {
                    let block = &blocks[0];
                    self.service
                        .process_block(block, &db_tx, &mut batch, &mut tx_pk_by_tx_hash_lru_cache)
                        .unwrap();
                }
                _ => {
                    panic!("Blocks vector is empty")
                }
            });

        // persist last height to db_tx if Some
        if let Some(block) = blocks.last() {
            self.service.persist_last_height(block.height(), &db_tx)?;
        }
        db_tx.commit().map_err(|e| e.into_string())?;
        Ok(())
    }

    pub(crate) fn get_last_height(&self) -> BlockHeight {
        self.service.get_last_height()
    }
}
