use crate::api::{BlockBatchIndexer, BlockProcessor, BlockchainClient, ChainSyncer, Height};
use crate::log;
use futures::future::join_all;
use futures::stream::StreamExt;
use min_batch::MinBatchExt;
use std::sync::Arc;

impl<B: Send + 'static, BH: 'static> ChainSyncer<B, BH> {
    pub fn new(
        client: Arc<dyn BlockchainClient<Block = B, BlockHash = BH> + Send + Sync>,
        processor: Arc<dyn BlockProcessor<Block = B> + Send + Sync>,
        indexer: Arc<dyn BlockBatchIndexer + Send + Sync>,
    ) -> Self {
        ChainSyncer {
            client,
            processor,
            indexer,
        }
    }

    pub async fn sync(
        &self,
        end_height: Height,
        min_batch_size: usize,
        optimal_batch_size: usize,
    ) -> () {
        let start_time = std::time::Instant::now();
        let last_height = self.indexer.get_last_height() + 1;
        let heights = last_height..=end_height;
        tokio_stream::iter(heights)
            .map(|height| {
                let rpc_client = self.client.clone();
                tokio::task::spawn_blocking(move || {
                    let (block, tx_count) = rpc_client
                        .get_block_with_tx_count_for_height(height)
                        .unwrap();
                    (height, block, tx_count)
                })
            })
            .buffered(256)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch(min_batch_size, optimal_batch_size, |(_, _, tx_count)| {
                *tx_count
            })
            .map(|blocks| {
                let processor = self.processor.clone();
                tokio::task::spawn_blocking(move || processor.process(blocks))
            })
            .buffered(512)
            .then(|res| async move {
                match res {
                    Ok(ci_blocks) => {
                        let tx_count =
                            ci_blocks.iter().fold(0, |acc, b| acc + b.1.txs.len()) as u64;
                        let indexer = self.indexer.clone();
                        let blocks = Arc::new(ci_blocks);
                        join_all(indexer.index(blocks)).await;
                        tx_count
                    }
                    Err(e) => panic!("Error: {:?}", e),
                }
            })
            .fold(
                (0 as u64, 0 as u64),
                |(total_tx_count, last_report_batches), tx_count| async move {
                    let total_time = start_time.elapsed().as_secs();
                    let txs_per_sec = format!("{:.1}", total_tx_count as f64 / total_time as f64);
                    if last_report_batches % 1000 == 0 {
                        log!(
                            "Processed {} with indexing Speed: {} txs/sec",
                            tx_count,
                            txs_per_sec
                        );
                        (total_tx_count + tx_count, 0)
                    } else {
                        (total_tx_count + tx_count, last_report_batches + 1)
                    }
                },
            )
            .await;
    }
}
