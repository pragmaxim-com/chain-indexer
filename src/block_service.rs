use crate::{
    api::TxService,
    codec_block,
    codec_tx::TxPkBytes,
    eutxo::eutxo_codec_utxo::UtxoPkBytes,
    info,
    model::{Block, BlockHash, BlockHeader, BlockHeight, O2oIndexValue, Transaction, TxHash},
    rocks_db_batch::{CustomFamilies, Families},
};
use lru::LruCache;
use rocksdb::{MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction};
use std::{cell::RefCell, num::NonZeroUsize};
use std::{rc::Rc, sync::Arc};

pub struct BlockService<'db, Tx: Transaction, CF: CustomFamilies<'db>> {
    pub(crate) tx_service: Arc<dyn TxService<'db, CF = CF, Tx = Tx>>,
    pub(crate) block_by_hash_lru_cache: RefCell<LruCache<BlockHash, Rc<Block<Tx>>>>,
    pub(crate) tx_pk_by_tx_hash_lru_cache: RefCell<LruCache<TxHash, TxPkBytes>>,
    pub(crate) utxo_pk_by_index_lru_cache: RefCell<LruCache<O2oIndexValue, UtxoPkBytes>>,
}

impl<'db, Tx: Transaction, CF: CustomFamilies<'db>> BlockService<'db, Tx, CF> {
    pub fn new(service: Arc<dyn TxService<'db, CF = CF, Tx = Tx>>) -> Self {
        BlockService {
            tx_service: service,
            block_by_hash_lru_cache: RefCell::new(LruCache::new(NonZeroUsize::new(1_000).unwrap())),
            tx_pk_by_tx_hash_lru_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
            utxo_pk_by_index_lru_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn persist_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();
        let mut utxo_pk_by_index_lru_cache = self.utxo_pk_by_index_lru_cache.borrow_mut();

        for block in blocks {
            self.persist_block(
                block,
                &mut block_height_by_hash_lru_cache,
                &mut tx_pk_by_tx_hash_lru_cache,
                &mut utxo_pk_by_index_lru_cache,
                db_tx,
                batch,
                families,
            )?;
        }

        Ok(())
    }

    pub(crate) fn persist_block(
        &self,
        block: Rc<Block<Tx>>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        self.tx_service.persist_txs(
            &block,
            db_tx,
            batch,
            tx_pk_by_tx_hash_lru_cache,
            utxo_pk_by_index_lru_cache,
            families,
        )?;
        self.persist_header(&block.header, db_tx, batch, families)?;
        block_height_by_hash_lru_cache.put(block.header.hash, Rc::clone(&block));
        Ok(())
    }

    pub(crate) fn remove_block(
        &self,
        block: Rc<Block<Tx>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, CF>,
    ) -> Result<(), String> {
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();
        let mut utxo_pk_by_index_lru_cache = self.utxo_pk_by_index_lru_cache.borrow_mut();

        for tx in block.txs.iter() {
            self.tx_service
                .remove_tx(
                    &block.header.height,
                    tx,
                    db_tx,
                    &mut tx_pk_by_tx_hash_lru_cache,
                    &mut utxo_pk_by_index_lru_cache,
                    families,
                )
                .map_err(|e| e.into_string())?;
        }
        self.remove_header(
            &block.header,
            db_tx,
            &mut block_height_by_hash_lru_cache,
            families,
        )
    }

    pub(crate) fn update_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, CF>,
    ) -> Result<Vec<Rc<Block<Tx>>>, String> {
        info!("Updating {} blocks", blocks.len());
        let removed_blocks: Result<Vec<Rc<Block<Tx>>>, String> = blocks
            .iter()
            .map(|block| {
                if let Some(block_to_remove) =
                    self.get_block_by_height(block.header.height, db_tx, families)?
                {
                    info!("Removing block {}", block_to_remove.header);
                    self.remove_block(Rc::clone(&block_to_remove), db_tx, families)?;
                    Ok(Some(block_to_remove))
                } else {
                    Ok(None)
                }
            })
            .filter_map(|result| match result {
                Ok(Some(foo)) => Some(Ok(foo)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect();

        info!("Persisting {} blocks in new fork", blocks.len());

        self.persist_blocks(blocks, db_tx, batch, families)
            .map_err(|e| e.into_string())?;

        removed_blocks
    }

    fn get_block_by_hash(
        &self,
        block_hash: &BlockHash,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, CF>,
    ) -> Result<Option<Rc<Block<Tx>>>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            Ok(Some(Rc::clone(value)))
        } else {
            let header_opt = self.get_block_header_by_hash(block_hash, db_tx, families)?;
            match header_opt {
                Some(block_header) => {
                    let txs =
                        self.tx_service
                            .get_txs_by_height(&block_header.height, db_tx, families)?;

                    Ok(Some(Rc::new(Block::new(block_header, txs))))
                }
                None => Ok(None),
            }
        }
    }

    fn get_block_by_height(
        &self,
        block_height: BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, CF>,
    ) -> Result<Option<Rc<Block<Tx>>>, rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        let hash_bytes = db_tx.get_cf(&families.shared.block_hash_by_pk_cf, height_bytes)?;

        if let Some(hash) = hash_bytes.map(|bytes| codec_block::bytes_to_block_hash(&bytes)) {
            self.get_block_by_hash(&hash, db_tx, families)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_block_header_by_hash(
        &self,
        block_hash: &BlockHash,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, CF>,
    ) -> Result<Option<BlockHeader>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            return Ok(Some(value.header.clone()));
        } else {
            let header_bytes = db_tx.get_cf(&families.shared.block_pk_by_hash_cf, block_hash)?;
            Ok(header_bytes.map(|bytes| codec_block::bytes_to_block_header(&bytes)))
        }
    }

    pub(crate) fn persist_header(
        &self,
        block_header: &BlockHeader,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);

        let header_bytes = codec_block::block_header_to_bytes(&block_header);
        batch.put_cf(
            &families.shared.block_hash_by_pk_cf,
            height_bytes,
            block_header.hash.0,
        );
        db_tx.put_cf(
            &families.shared.block_pk_by_hash_cf,
            block_header.hash.0,
            header_bytes,
        )?;

        Ok(())
    }

    pub(crate) fn remove_header(
        &self,
        block_header: &BlockHeader,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
        families: &Families<'db, CF>,
    ) -> Result<(), String> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);

        db_tx
            .delete_cf(&families.shared.block_hash_by_pk_cf, height_bytes)
            .map_err(|e| e.to_string())?;

        db_tx
            .delete_cf(&families.shared.block_pk_by_hash_cf, block_header.hash.0)
            .map_err(|e| e.to_string())?;

        block_height_by_hash_lru_cache.pop(&block_header.hash);

        Ok(())
    }
}
