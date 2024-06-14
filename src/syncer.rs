use crate::api::{BlockProcessor, BlockchainClient, ChainSyncer, Height, Storage};
use crate::log;
use broadcast_sink::StreamBroadcastSinkExt;
use futures::stream::StreamExt;
use min_batch::MinBatchExt;
use std::sync::{Arc, Mutex};

impl<B: Send + 'static, BH: 'static> ChainSyncer<B, BH> {
    pub fn new(
        client: Arc<dyn BlockchainClient<Block = B, BlockHash = BH> + Send + Sync>,
        processor: Arc<dyn BlockProcessor<Block = B> + Send + Sync>,
        storage: Arc<dyn Storage + Send + Sync>,
    ) -> Self {
        ChainSyncer {
            client,
            processor,
            storage,
        }
    }

    pub async fn sync(&self, end_height: Height, min_batch_size: usize) -> () {
        let start_time = std::time::Instant::now();
        let total_tx_count = Arc::new(Mutex::new(0));
        let last_height = self.storage.get_last_height() + 1;
        let heights = last_height..=end_height;
        tokio_stream::iter(heights)
            .map(|height| {
                let rpc_client = Arc::clone(&self.client);
                let total_tx_count = Arc::clone(&total_tx_count);
                tokio::task::spawn_blocking(move || {
                    let (block, tx_count) = rpc_client
                        .get_block_with_tx_count_for_height(height)
                        .unwrap();

                    let total_time = start_time.elapsed().as_secs();
                    let mut total_tx_count = total_tx_count.lock().unwrap();
                    *total_tx_count += tx_count;
                    let txs_per_sec = format!("{:.1}", *total_tx_count as f64 / total_time as f64);
                    if height % 1000 == 0 {
                        log!(
                            "Processed {} txs with indexing Speed: {} txs/sec",
                            *total_tx_count,
                            txs_per_sec
                        );
                    }

                    (height, block, tx_count)
                })
            })
            .buffered(256)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch(min_batch_size, |(_, _, tx_count)| *tx_count)
            .map(|blocks| {
                let processor = Arc::clone(&self.processor);
                tokio::task::spawn_blocking(move || processor.process(blocks))
            })
            .buffered(512)
            .map(|res| match res {
                Ok(blocks) => blocks,
                Err(e) => panic!("Error: {:?}", e),
            })
            .broadcast(100, self.storage.get_indexers())
            .await;
    }
}
