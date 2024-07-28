use crate::{
    api::TxService,
    codec_block,
    codec_tx::TxPkBytes,
    info,
    model::{Block, BlockHash, BlockHeader, BlockHeight, Transaction, TxHash},
    rocks_db_batch::{ChainFamilies, RocksDbBatch},
};
use lru::LruCache;
use std::{cell::RefCell, num::NonZeroUsize, sync::Arc};
use std::{cell::RefMut, rc::Rc};

pub struct BlockService<'db, Tx: Transaction, CF: ChainFamilies<'db>> {
    pub(crate) tx_service: Arc<dyn TxService<'db, CF = CF, Tx = Tx>>,
    pub(crate) block_by_hash_lru_cache: RefCell<LruCache<BlockHash, Rc<Block<Tx>>>>,
    pub(crate) tx_pk_by_tx_hash_lru_cache: RefCell<LruCache<TxHash, TxPkBytes>>,
}

impl<'db, Tx: Transaction, CF: ChainFamilies<'db>> BlockService<'db, Tx, CF> {
    pub fn new(service: Arc<dyn TxService<'db, CF = CF, Tx = Tx>>) -> Self {
        BlockService {
            tx_service: service,
            block_by_hash_lru_cache: RefCell::new(LruCache::new(NonZeroUsize::new(1_000).unwrap())),
            tx_pk_by_tx_hash_lru_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(10_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn persist_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();

        for block in blocks {
            self.persist_block(
                block,
                &mut block_height_by_hash_lru_cache,
                &mut tx_pk_by_tx_hash_lru_cache,
                batch,
            )?;
        }

        Ok(())
    }

    pub(crate) fn persist_block(
        &self,
        block: Rc<Block<Tx>>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        self.tx_service
            .persist_txs(&block, batch, tx_pk_by_tx_hash_lru_cache)?;
        self.persist_header(&block.header, batch)?;
        block_height_by_hash_lru_cache.put(block.header.hash, Rc::clone(&block));
        Ok(())
    }

    pub(crate) fn remove_block(
        &self,
        block: Rc<Block<Tx>>,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<(), String> {
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();

        for tx in block.txs.iter() {
            self.tx_service
                .remove_tx(
                    &block.header.height,
                    tx,
                    batch,
                    &mut tx_pk_by_tx_hash_lru_cache,
                )
                .map_err(|e| e.into_string())?;
        }
        self.remove_header(&block.header, batch, &mut block_height_by_hash_lru_cache)
    }

    pub(crate) fn update_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<Vec<Rc<Block<Tx>>>, String> {
        info!("Updating {} blocks", blocks.len());
        let removed_blocks: Result<Vec<Rc<Block<Tx>>>, String> = blocks
            .iter()
            .map(|block| {
                if let Some(block_to_remove) =
                    self.get_block_by_height(block.header.height, batch)?
                {
                    info!("Removing block {}", block_to_remove.header);
                    self.remove_block(Rc::clone(&block_to_remove), batch)?;
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

        self.persist_blocks(blocks, batch)
            .map_err(|e| e.into_string())?;

        removed_blocks
    }

    fn get_block_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<Option<Rc<Block<Tx>>>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            Ok(Some(Rc::clone(value)))
        } else {
            let header_opt = self.get_block_header_by_hash(block_hash, batch)?;
            match header_opt {
                Some(block_header) => {
                    let txs = self
                        .tx_service
                        .get_txs_by_height(&block_header.height, batch)?;

                    Ok(Some(Rc::new(Block::new(block_header, txs))))
                }
                None => Ok(None),
            }
        }
    }

    fn get_block_by_height(
        &self,
        block_height: BlockHeight,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<Option<Rc<Block<Tx>>>, rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        let hash_bytes = batch
            .db_tx
            .borrow()
            .get_cf(batch.shared.block_hash_by_pk_cf, height_bytes)?;

        if let Some(hash) = hash_bytes.map(|bytes| codec_block::bytes_to_block_hash(&bytes)) {
            self.get_block_by_hash(&hash, batch)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_block_header_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<Option<BlockHeader>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            return Ok(Some(value.header.clone()));
        } else {
            let header_bytes = batch
                .db_tx
                .borrow()
                .get_cf(batch.shared.block_pk_by_hash_cf, block_hash)?;
            Ok(header_bytes.map(|bytes| codec_block::bytes_to_block_header(&block_hash.0, &bytes)))
        }
    }

    pub(crate) fn persist_header(
        &self,
        block_header: &BlockHeader,
        batch: &RocksDbBatch<'db, CF>,
    ) -> Result<(), rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);
        let block_hash_by_pk_cf = batch.shared.block_hash_by_pk_cf;

        let header_bytes = codec_block::block_header_to_bytes(&block_header);
        batch
            .batch
            .borrow_mut()
            .put_cf(&block_hash_by_pk_cf, height_bytes, block_header.hash.0);
        batch.db_tx.borrow().put_cf(
            batch.shared.block_pk_by_hash_cf,
            block_header.hash.0,
            header_bytes,
        )?;

        Ok(())
    }

    pub(crate) fn remove_header(
        &self,
        block_header: &BlockHeader,
        batch: &RocksDbBatch<'db, CF>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
    ) -> Result<(), String> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);
        let block_hash_by_pk_cf = batch.shared.block_hash_by_pk_cf;

        batch
            .db_tx
            .borrow()
            .delete_cf(&block_hash_by_pk_cf, height_bytes)
            .map_err(|e| e.to_string())?;

        batch
            .db_tx
            .borrow()
            .delete_cf(batch.shared.block_pk_by_hash_cf, block_header.hash.0)
            .map_err(|e| e.to_string())?;

        block_height_by_hash_lru_cache.pop(&block_header.hash);

        Ok(())
    }
}
