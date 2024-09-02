use crate::{
    api::{ServiceError, TxReadService},
    codec_block, info,
    persistence::Persistence,
    rocks_db_batch::CustomFamilies,
};
use lru::LruCache;
use model::{Block, BlockHash, BlockHeader, BlockHeight};
use rocksdb::IteratorMode;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

pub struct BlockReadService<Tx, CF: CustomFamilies> {
    pub(crate) storage: Arc<Persistence<CF>>,
    pub(crate) tx_service: Arc<dyn TxReadService<CF = CF, Tx = Tx> + Send + Sync>,
    pub(crate) block_by_hash_cache: Arc<Mutex<LruCache<BlockHash, Arc<Block<Tx>>>>>,
}

impl<Tx, CF: CustomFamilies> BlockReadService<Tx, CF> {
    pub(crate) fn new(
        storage: Arc<Persistence<CF>>,
        tx_service: Arc<dyn TxReadService<CF = CF, Tx = Tx> + Send + Sync>,
    ) -> Self {
        BlockReadService {
            storage,
            tx_service,
            block_by_hash_cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(1_000).unwrap(),
            ))),
        }
    }
    pub(crate) fn get_block_by_hash(
        &self,
        block_hash: &BlockHash,
    ) -> Result<Option<Arc<Block<Tx>>>, ServiceError> {
        let mut cache = self.block_by_hash_cache.lock().unwrap();
        if let Some(value) = cache.get(block_hash) {
            Ok(Some(Arc::clone(value)))
        } else {
            let header_opt = self.get_block_header_by_hash(block_hash)?;
            match header_opt {
                Some(block_header) => {
                    let txs = self.tx_service.get_txs_by_height(&block_header.height)?;
                    let block = Arc::new(Block::new(block_header, txs, 0)); // TODO weight ?
                    cache.put(block_hash.clone(), block.clone());
                    Ok(Some(block))
                }
                None => {
                    info!("Not found {}", block_hash);
                    Ok(None)
                }
            }
        }
    }

    pub(crate) fn get_latest_block(&self) -> Result<Option<Arc<Block<Tx>>>, ServiceError> {
        let latest_block = self
            .storage
            .db
            .iterator_cf(
                &self.storage.families.shared.block_hash_by_pk_cf,
                IteratorMode::End,
            )
            .take(1)
            .map(|result| {
                result
                    .map_err(|err| ServiceError::new(&err.to_string()))
                    .and_then(|(_, hash_bytes)| {
                        let block_hash = codec_block::bytes_to_block_hash(&hash_bytes);
                        info!("Received http request for block {}", block_hash);
                        self.get_block_by_hash(&block_hash)
                    })
            })
            .collect::<Result<Vec<Option<Arc<Block<Tx>>>>, ServiceError>>()?
            .into_iter()
            .find_map(|option| option);
        info!("Found but {}", latest_block.is_some());
        Ok(latest_block)
    }

    pub(crate) fn get_block_by_height(
        &self,
        block_height: BlockHeight,
    ) -> Result<Option<Arc<Block<Tx>>>, ServiceError> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        let hash_bytes = self.storage.db.get_cf(
            &self.storage.families.shared.block_hash_by_pk_cf,
            height_bytes,
        )?;
        if let Some(hash) = hash_bytes.map(|bytes| codec_block::bytes_to_block_hash(&bytes)) {
            self.get_block_by_hash(&hash)
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_block_header_by_hash(
        &self,
        block_hash: &BlockHash,
    ) -> Result<Option<BlockHeader>, ServiceError> {
        if let Some(value) = self.block_by_hash_cache.lock().unwrap().get(block_hash) {
            Ok(Some(value.header.clone()))
        } else {
            let header_bytes = self.storage.db.get_cf(
                &self.storage.families.shared.block_pk_by_hash_cf,
                block_hash,
            )?;
            Ok(header_bytes.map(|bytes| codec_block::bytes_to_block_header(&bytes)))
        }
    }
}
