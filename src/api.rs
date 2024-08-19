use std::{pin::Pin, sync::Arc};

use crate::{
    codec_tx::TxPkBytes,
    eutxo::{eutxo_codec_utxo::UtxoPkBytes, eutxo_schema::DbSchema},
    model::{
        AssetId, BatchWeight, Block, BlockHeader, BlockHeight, BoxWeight, O2mIndexValue,
        O2oIndexValue, TxCount, TxHash,
    },
    rocks_db_batch::{CustomFamilies, Families},
};
use async_trait::async_trait;
use futures::Stream;
use lru::LruCache;
use rocksdb::{MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction};

pub trait BlockProcessor {
    type FromBlock: Send;
    type IntoTx: Send;

    fn process_block(&self, block: &Self::FromBlock) -> Result<Block<Self::IntoTx>, String>;

    fn process_batch(
        &self,
        block_batch: &[Self::FromBlock],
        tx_count: TxCount,
    ) -> Result<(Vec<Block<Self::IntoTx>>, TxCount), String>;
}

pub trait IoProcessor<FromInput, IntoInput, FromOutput, IntoOutput> {
    fn process_inputs(&self, ins: &[FromInput]) -> Vec<IntoInput>;
    fn process_outputs(&self, outs: &[FromOutput]) -> (BoxWeight, Vec<IntoOutput>);
}

#[async_trait]
pub trait BlockProvider {
    type OutTx: Send;

    fn get_schema(&self) -> DbSchema;

    fn get_processed_block(&self, header: BlockHeader) -> Result<Block<Self::OutTx>, String>;

    async fn get_chain_tip(&self) -> Result<BlockHeader, String>;

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<Self::OutTx>>, BatchWeight)> + Send + 'life0>>;
}

pub trait TxService<'db> {
    type CF: CustomFamilies<'db>;
    type Tx;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, Self::CF>,
    ) -> Result<Vec<Self::Tx>, rocksdb::Error>;

    #[warn(clippy::too_many_arguments)]
    fn persist_txs(
        &self,
        block: &Block<Self::Tx>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &Self::Tx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, Self::CF>,
    ) -> Result<(), rocksdb::Error>;
}

pub trait BlockMonitor<Tx> {
    fn monitor(&self, block_batch: &[Block<Tx>], batch_weight: &BatchWeight);
}

pub struct Storage {
    pub db: Arc<OptimisticTransactionDB<MultiThreaded>>,
}
