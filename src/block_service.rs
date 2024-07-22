use crate::{
    api::TxService,
    codec_block,
    codec_tx::TxPkBytes,
    model::{Block, BlockHash, BlockHeader, BlockHeight, Transaction, TxHash},
    rocks_db_batch::RocksDbBatch,
};
use lru::LruCache;
use std::cell::RefMut;
use std::{cell::RefCell, num::NonZeroUsize, sync::Arc};

pub struct BlockService<Tx: Transaction + Clone> {
    pub(crate) tx_service: Arc<dyn TxService<Tx = Tx>>,
    pub(crate) block_by_hash_lru_cache: RefCell<LruCache<BlockHash, Block<Tx>>>,
    pub(crate) tx_pk_by_tx_hash_lru_cache: RefCell<LruCache<TxHash, TxPkBytes>>,
}

impl<Tx: Transaction + Clone> BlockService<Tx> {
    pub fn new(service: Arc<dyn TxService<Tx = Tx>>) -> Self {
        BlockService {
            tx_service: service,
            block_by_hash_lru_cache: RefCell::new(LruCache::new(NonZeroUsize::new(1_000).unwrap())),
            tx_pk_by_tx_hash_lru_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(10_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn persist_block(
        &self,
        block: &Block<Tx>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String> {
        let mut batch = batch.borrow_mut();
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();

        for tx in block.txs.iter() {
            self.tx_service
                .persist_tx(
                    &block.header.height,
                    tx,
                    &mut batch,
                    &mut tx_pk_by_tx_hash_lru_cache,
                )
                .map_err(|e| e.into_string())?;
        }
        self.persist_header(&block, &mut batch, &mut block_height_by_hash_lru_cache)?;
        Ok(())
    }

    pub(crate) fn remove_block(
        &self,
        block: &Block<Tx>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String> {
        let mut batch = batch.borrow_mut();
        let mut tx_pk_by_tx_hash_lru_cache = self.tx_pk_by_tx_hash_lru_cache.borrow_mut();
        let mut block_height_by_hash_lru_cache = self.block_by_hash_lru_cache.borrow_mut();

        for tx in block.txs.iter() {
            self.tx_service
                .remove_tx(
                    &block.header.height,
                    tx,
                    &mut batch,
                    &mut tx_pk_by_tx_hash_lru_cache,
                )
                .map_err(|e| e.into_string())?;
        }
        self.remove_header(
            &block.header,
            &mut batch,
            &mut block_height_by_hash_lru_cache,
        )
    }

    pub(crate) fn update_blocks(
        &self,
        blocks: &Vec<Block<Tx>>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Vec<Block<Tx>>, String> {
        let removed_blocks: Result<Vec<Block<Tx>>, String> = blocks
            .iter()
            .map(|block| {
                if let Some(block_to_remove) =
                    self.get_block_by_height(block.header.height, batch)?
                {
                    self.remove_block(&block_to_remove, batch)?;
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

        for block in blocks.iter() {
            self.persist_block(block, batch)?;
        }

        removed_blocks
    }

    fn get_block_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<Block<Tx>>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            Ok(Some(value.clone()))
        } else {
            let header_opt = self.get_block_header_by_hash(block_hash, batch)?;
            match header_opt {
                Some(block_header) => {
                    let txs = self
                        .tx_service
                        .get_txs_by_height(&block_header.height, batch)?;

                    Ok(Some(Block::new(block_header, txs)))
                }
                None => Ok(None),
            }
        }
    }

    fn get_block_by_height(
        &self,
        block_height: BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<Block<Tx>>, rocksdb::Error> {
        let mut_batch = batch.borrow_mut();
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        let hash_bytes = mut_batch
            .db_tx
            .get_cf(mut_batch.block_hash_by_pk_cf, height_bytes)?;

        if let Some(hash) = hash_bytes.map(|bytes| codec_block::bytes_to_block_hash(&bytes)) {
            self.get_block_by_hash(&hash, batch)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_block_header_by_hash(
        &self,
        block_hash: &BlockHash,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Option<BlockHeader>, rocksdb::Error> {
        if let Some(value) = self.block_by_hash_lru_cache.borrow_mut().get(block_hash) {
            return Ok(Some(value.header));
        } else {
            let batch = batch.borrow_mut();
            let header_bytes = batch.db_tx.get_cf(batch.block_pk_by_hash_cf, block_hash)?;
            Ok(header_bytes.map(|bytes| codec_block::bytes_to_block_header(&block_hash.0, &bytes)))
        }
    }

    pub(crate) fn persist_header(
        &self,
        block: &Block<Tx>,
        batch: &mut RefMut<RocksDbBatch>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Block<Tx>>,
    ) -> Result<(), String> {
        let block_header = block.header;
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);
        let block_hash_by_pk_cf = batch.block_hash_by_pk_cf;

        let header_bytes = codec_block::block_header_to_bytes(&block_header);
        batch
            .batch
            .put_cf(&block_hash_by_pk_cf, height_bytes, block_header.hash.0);
        batch
            .db_tx
            .put_cf(batch.block_pk_by_hash_cf, block_header.hash.0, header_bytes)
            .map_err(|e| e.to_string())?;

        block_height_by_hash_lru_cache.put(block.header.hash, block.clone());

        Ok(())
    }

    pub(crate) fn remove_header(
        &self,
        block_header: &BlockHeader,
        batch: &mut RefMut<RocksDbBatch>,
        block_height_by_hash_lru_cache: &mut LruCache<BlockHash, Block<Tx>>,
    ) -> Result<(), String> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);
        let block_hash_by_pk_cf = batch.block_hash_by_pk_cf;

        batch
            .db_tx
            .delete_cf(&block_hash_by_pk_cf, height_bytes)
            .map_err(|e| e.to_string())?;

        batch
            .db_tx
            .delete_cf(batch.block_pk_by_hash_cf, block_header.hash.0)
            .map_err(|e| e.to_string())?;

        block_height_by_hash_lru_cache.pop(&block_header.hash);

        Ok(())
    }
}
