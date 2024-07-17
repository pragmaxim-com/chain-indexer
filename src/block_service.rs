use crate::{
    api::TxService,
    codec_block,
    codec_tx::TxPkBytes,
    indexer::RocksDbBatch,
    model::{Block, BlockHash, BlockHeader, BlockHeight, Transaction, TxHash},
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
        block: Block<Tx>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String> {
        let mut batch = batch.borrow_mut();
        self.persist_header(&block.header, &mut batch)?;
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
            self.tx_service
                .persist_outputs(&block.header.height, tx, &mut batch);
            if !tx.is_coinbase() {
                self.tx_service.persist_inputs(
                    &block.header.height,
                    tx,
                    &mut batch,
                    &mut tx_pk_by_tx_hash_lru_cache,
                );
            }
        }
        block_height_by_hash_lru_cache.put(block.header.hash, block);
        Ok(())
    }

    pub(crate) fn update_blocks(
        &self,
        blocks: &Vec<Block<Tx>>,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<(), String> {
        todo!("s")
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
        block_header: &BlockHeader,
        batch: &mut RefMut<RocksDbBatch>,
    ) -> Result<(), String> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);
        let block_hash_by_pk_cf = batch.block_hash_by_pk_cf;
        batch
            .batch
            .put_cf(&block_hash_by_pk_cf, height_bytes, block_header.hash.0);

        let header_bytes = codec_block::block_header_to_bytes(&block_header);
        batch
            .db_tx
            .put_cf(batch.block_pk_by_hash_cf, block_header.hash.0, header_bytes)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
