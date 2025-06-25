use super::{
    btc_block_processor::BtcBlockProcessor,
    btc_client::BtcClient,
    btc_io_processor::BtcIoProcessor,
};
use crate::eutxo::eutxo_model::{Block, BlockHeader, BlockHeight};
use crate::model::BatchWeight;
use crate::{
    api::{BlockProcessor, BlockProvider, ServiceError},
    info,
    settings::BitcoinConfig,
};
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::Stream;
use min_batch::ext::MinBatchExt;
use redb::ReadTransaction;
use std::{pin::Pin, sync::Arc};

pub struct BtcBlockProvider {
    pub client: Arc<BtcClient>,
    pub processor: Arc<BtcBlockProcessor>,
    pub db: Arc<redb::Database>,
}

impl BtcBlockProvider {
    pub fn new(bitcoin_config: &BitcoinConfig, db: Arc<redb::Database>) -> Self {
        BtcBlockProvider {
            client: Arc::new(BtcClient::new(bitcoin_config)),
            processor: Arc::new(BtcBlockProcessor::new(BtcIoProcessor { } )),
            db
        }
    }
}

#[async_trait]
impl BlockProvider for BtcBlockProvider {

    fn get_processed_block(&self, header: BlockHeader, read_tx: &ReadTransaction) -> Result<Block, ServiceError> {
        let block = self.client.get_block_by_hash(header.hash)?;
        self.processor.process_block(&block, read_tx)
    }

    async fn get_chain_tip(&self, read_tx: &ReadTransaction) -> Result<BlockHeader, ServiceError> {
        self.client
            .get_best_block()
            .and_then(|b| self.processor.process_block(&b, read_tx))
            .map(|b| b.header)
    }

    async fn stream(
        &self,
        chain_tip_header: BlockHeader,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
        fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block>, BatchWeight)> + Send + 'life0>> {
        let last_height = last_header.map_or(0, |h| h.id.0);
        info!("Indexing from {:?} to {:?}", last_height, chain_tip_header);
        let heights = last_height..=chain_tip_header.id.0;

        tokio_stream::iter(heights)
            .map(|height| {
                let client = Arc::clone(&self.client);
                tokio::task::spawn_blocking(move || {
                    client.get_block_by_height(BlockHeight(height)).unwrap()
                })
            })
            .buffered(fetching_par)
            .map(|res| match res {
                Ok(block) => {
                    let db = Arc::clone(&self.db);
                    let read_tx = db.begin_read().unwrap();
                    self.processor.process_block(&block, &read_tx).unwrap()
                }
                Err(e) => panic!("Error: {:?}", e),
            })
/*            .buffered(processing_par)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
*/            .min_batch_with_weight(min_batch_size, |block| block.weight as usize)
            .boxed()
    }
}
