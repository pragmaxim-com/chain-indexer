use backend::api::BlockProvider;
use backend::{
    block_read_service::BlockReadService,
    block_write_service::BlockWriteService,
    eutxo::{
        ergo::ergo_block_provider::ErgoBlockProvider, eutxo_schema::DbSchema, eutxo_storage,
        eutxo_tx_read_service::EuTxReadService, eutxo_tx_write_service::EuTxWriteService,
    },
    persistence::Persistence,
    settings::AppConfig,
};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use model::{BlockHash, BlockHeader, BlockHeight, BlockTimestamp};
    use rocksdb::{
        MultiThreaded, OptimisticTransactionDB, OptimisticTransactionOptions, WriteOptions,
    };

    use super::*;

    #[test]
    fn test_block_write_service() {
        let app_config = AppConfig::new("../config/settings").unwrap();
        let schema = DbSchema::load_config("../config/schema.yaml");
        let block_provider = ErgoBlockProvider::new(&app_config.ergo, schema.ergo);
        let db_path: String = format!("{}/{}/{}", app_config.indexer.db_path, "test", "ergo");
        let db_shema = block_provider.get_schema();
        let db = Arc::new(eutxo_storage::get_db(&db_shema, &db_path));
        let storage = Arc::new(Persistence::new(Arc::clone(&db), &db_shema));
        let tx_read_service = Arc::new(EuTxReadService::new(Arc::clone(&storage)));
        let block_read_service = Arc::new(BlockReadService::new(
            Arc::clone(&storage),
            tx_read_service.clone(),
        ));
        let block_write_service = BlockWriteService::new(
            Arc::new(EuTxWriteService::new(false)),
            Arc::clone(&block_read_service),
        );

        for height in 1..11 {
            let block_hashes = block_provider
                .client
                .get_block_ids_by_height_sync(BlockHeight(height))
                .unwrap();

            let header = BlockHeader {
                height: BlockHeight(height),
                timestamp: BlockTimestamp(111),
                hash: block_hashes
                    .first()
                    .map(|h| BlockHash::from(h.clone()))
                    .unwrap(),
                prev_hash: BlockHash::from(
                    "cc3c4d42d2b3ac93385e2dc45c0036ad638e16880c7e2c3271b4efb8cc355f93".to_string(),
                ),
            };
            let block = Arc::new(block_provider.get_processed_block(header.clone()).unwrap());

            let mut write_options = WriteOptions::default();
            write_options.disable_wal(false);

            let db_tx: rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>> =
                db.transaction_opt(&write_options, &OptimisticTransactionOptions::default());

            let mut batch = db_tx.get_writebatch();

            block_write_service
                .persist_blocks(vec![block], &db_tx, &mut batch, &storage.families)
                .unwrap();

            db.write(batch).unwrap();
            db_tx.commit().unwrap();

            let block = block_read_service.get_block_by_hash(&header.hash).unwrap();
            assert_eq!(block.unwrap().header.height.0, height)
        }
    }
}
