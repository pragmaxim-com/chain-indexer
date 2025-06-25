use backend::api::BlockProvider;
use backend::settings::AppConfig;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use backend::eutxo::btc::btc_block_provider::BtcBlockProvider;
    use backend::eutxo::eutxo_model::{Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp};
    use backend::eutxo::eutxo_storage;
    use std::env::temp_dir;

    #[test]
    fn test_block_write_service() {
        let app_config = AppConfig::new("../config/settings").unwrap();
        let db = Arc::new(eutxo_storage::get_db(temp_dir().join("btc")).unwrap());
        let block_provider = BtcBlockProvider::new(&app_config.bitcoin, Arc::clone(&db));

        for height in 1..11 {
            let block = block_provider
                .client
                .get_block_by_height(BlockHeight(height))
                .unwrap();

            let header = BlockHeader {
                id: BlockHeight(height),
                hash: BlockHash(*block.underlying.block_hash().as_ref()),
                prev_hash: BlockHash(*block.underlying.header.prev_blockhash.as_ref()),
                timestamp: BlockTimestamp(111),
            };
            let read_tx = db.begin_read().unwrap();
            let write_tx = db.begin_write().unwrap();
            let block = Arc::new(block_provider.get_processed_block(header.clone(), &read_tx).unwrap());

            Block::store(&write_tx, &block).unwrap();

            write_tx.commit().unwrap();
            assert_eq!(block.header.id.0, height)
        }
    }
}
