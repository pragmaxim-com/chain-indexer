use crate::{
    api::{ServiceError, TxWriteService},
    block_read_service::BlockReadService,
    codec_block,
    codec_tx::TxPkBytes,
    eutxo::eutxo_codec_utxo::UtxoPkBytes,
    info,
    rocks_db_batch::{CustomFamilies, Families},
};
use lru::LruCache;
use model::{AssetId, Block, BlockHash, BlockHeader, O2mIndexValue, O2oIndexValue, TxHash};
use rocksdb::{MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction};
use std::{cell::RefCell, num::NonZeroUsize};
use std::{rc::Rc, sync::Arc};

pub struct BlockWriteService<Tx, CF: CustomFamilies> {
    pub(crate) tx_service: Arc<dyn TxWriteService<CF = CF, Tx = Tx>>,
    pub(crate) block_service: Arc<BlockReadService<Tx, CF>>,
    pub(crate) block_by_hash_cache: RefCell<LruCache<BlockHash, Rc<Block<Tx>>>>,
    pub(crate) tx_pk_by_tx_hash_cache: RefCell<LruCache<TxHash, TxPkBytes>>,
    pub(crate) utxo_pk_by_index_cache: RefCell<LruCache<O2oIndexValue, UtxoPkBytes>>,
    pub(crate) utxo_birth_pk_by_index_cache: RefCell<LruCache<O2mIndexValue, Vec<u8>>>,
    pub(crate) asset_birth_pk_by_asset_id_cache: RefCell<LruCache<AssetId, Vec<u8>>>,
}

impl<Tx, CF: CustomFamilies> BlockWriteService<Tx, CF> {
    pub fn new(
        tx_service: Arc<dyn TxWriteService<CF = CF, Tx = Tx>>,
        block_service: Arc<BlockReadService<Tx, CF>>,
    ) -> Self {
        BlockWriteService {
            tx_service,
            block_service,
            block_by_hash_cache: RefCell::new(LruCache::new(NonZeroUsize::new(1_000).unwrap())),
            tx_pk_by_tx_hash_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
            utxo_pk_by_index_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
            utxo_birth_pk_by_index_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
            asset_birth_pk_by_asset_id_cache: RefCell::new(LruCache::new(
                NonZeroUsize::new(5_000_000).unwrap(),
            )),
        }
    }

    pub(crate) fn persist_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<CF>,
    ) -> Result<(), rocksdb::Error> {
        let mut tx_pk_by_tx_hash_cache = self.tx_pk_by_tx_hash_cache.borrow_mut();
        let mut block_by_hash_cache = self.block_by_hash_cache.borrow_mut();
        let mut utxo_pk_by_index_cache = self.utxo_pk_by_index_cache.borrow_mut();
        let mut utxo_birth_pk_by_index_cache = self.utxo_birth_pk_by_index_cache.borrow_mut();
        let mut asset_birth_pk_by_asset_id_cache =
            self.asset_birth_pk_by_asset_id_cache.borrow_mut();

        for block in blocks {
            self.persist_block(
                block,
                &mut block_by_hash_cache,
                &mut tx_pk_by_tx_hash_cache,
                &mut utxo_pk_by_index_cache,
                &mut utxo_birth_pk_by_index_cache,
                &mut asset_birth_pk_by_asset_id_cache,
                db_tx,
                batch,
                families,
            )?;
        }

        Ok(())
    }

    #[warn(clippy::too_many_arguments)]
    pub(crate) fn persist_block(
        &self,
        block: Rc<Block<Tx>>,
        block_by_hash_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<CF>,
    ) -> Result<(), rocksdb::Error> {
        self.tx_service.persist_txs(
            &block,
            db_tx,
            batch,
            tx_pk_by_tx_hash_cache,
            utxo_pk_by_index_cache,
            utxo_birth_pk_by_index_cache,
            asset_birth_pk_by_asset_id_cache,
            families,
        )?;
        self.persist_header(&block.header, db_tx, batch, families)?;
        block_by_hash_cache.put(block.header.hash, Rc::clone(&block));
        Ok(())
    }

    pub(crate) fn remove_block(
        &self,
        block: Arc<Block<Tx>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<CF>,
    ) -> Result<(), ServiceError> {
        let mut tx_pk_by_tx_hash_cache = self.tx_pk_by_tx_hash_cache.borrow_mut();
        let mut block_by_hash_cache = self.block_by_hash_cache.borrow_mut();
        let mut utxo_pk_by_index_cache = self.utxo_pk_by_index_cache.borrow_mut();

        for tx in block.txs.iter() {
            self.tx_service.remove_tx(
                &block.header.height,
                tx,
                db_tx,
                &mut tx_pk_by_tx_hash_cache,
                &mut utxo_pk_by_index_cache,
                families,
            )?;
        }
        self.remove_header(&block.header, db_tx, &mut block_by_hash_cache, families)
    }

    pub(crate) fn update_blocks(
        &self,
        blocks: Vec<Rc<Block<Tx>>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<CF>,
    ) -> Result<Vec<Arc<Block<Tx>>>, ServiceError> {
        info!("Updating {} blocks", blocks.len());
        let removed_blocks: Result<Vec<Arc<Block<Tx>>>, ServiceError> = blocks
            .iter()
            .map(|block| {
                if let Some(block_to_remove) = self
                    .block_service
                    .get_block_by_height(block.header.height)?
                {
                    info!("Removing block {}", block_to_remove.header);
                    self.remove_block(Arc::clone(&block_to_remove), db_tx, families)?;
                    Ok(Some(block_to_remove))
                } else {
                    Ok(None)
                }
            })
            .filter_map(|result| match result {
                Ok(Some(block)) => Some(Ok(block)),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            })
            .collect();

        info!("Persisting {} blocks in new fork", blocks.len());

        self.persist_blocks(blocks, db_tx, batch, families)?;

        removed_blocks
    }
    pub(crate) fn persist_header(
        &self,
        block_header: &BlockHeader,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<CF>,
    ) -> Result<(), rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);

        let header_bytes = codec_block::block_header_to_bytes(block_header);
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
        block_by_hash_cache: &mut LruCache<BlockHash, Rc<Block<Tx>>>,
        families: &Families<CF>,
    ) -> Result<(), ServiceError> {
        let height_bytes = codec_block::block_height_to_bytes(&block_header.height);

        db_tx.delete_cf(&families.shared.block_hash_by_pk_cf, height_bytes)?;

        db_tx.delete_cf(&families.shared.block_pk_by_hash_cf, block_header.hash.0)?;

        block_by_hash_cache.pop(&block_header.hash);

        Ok(())
    }
}
