use crate::api::{BlockProcessor, ServiceError};
use crate::model::{BatchWeight, Block, BlockHeader};
use crate::{
    api::BlockProvider,
    eutxo::{eutxo_model::EuTx, eutxo_schema::DbSchema},
    settings::CardanoConfig,
};
use min_batch::ext::MinBatchExt;
use pallas::network::miniprotocols::chainsync::NextResponse;
use pallas::network::miniprotocols::Point;
use std::{pin::Pin, sync::Arc};
use tokio::runtime::Runtime;

use crate::info;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use super::{
    cardano_block_processor::{CardanoBlockProcessor, GENESIS_START_TIME},
    cardano_client::{CardanoClient, CBOR},
    cardano_io_processor::CardanoIoProcessor,
};

pub struct CardanoBlockProvider {
    pub client: CardanoClient,
    pub processor: Arc<CardanoBlockProcessor>,
}

impl CardanoBlockProvider {
    pub async fn new(cardano_config: &CardanoConfig, db_schema: DbSchema) -> Self {
        CardanoBlockProvider {
            client: CardanoClient::new(cardano_config).await,
            processor: Arc::new(CardanoBlockProcessor::new(CardanoIoProcessor::new(
                db_schema,
            ))),
        }
    }
}

#[async_trait]
impl BlockProvider for CardanoBlockProvider {
    type OutTx = EuTx;

    fn get_schema(&self) -> DbSchema {
        self.processor.io_processor.db_schema.clone()
    }

    async fn get_chain_tip(&self) -> Result<BlockHeader, ServiceError> {
        let header = self
            .client
            .get_best_block()
            .await
            .and_then(|b| self.processor.process_block(&b))
            .map(|b| b.header)?;
        Ok(header)
    }

    fn get_processed_block(&self, h: BlockHeader) -> Result<Block<Self::OutTx>, ServiceError> {
        let point = Point::new(
            (h.timestamp.0 - GENESIS_START_TIME) as u64,
            h.hash.0.to_vec(),
        );
        let rt = Runtime::new().unwrap();
        let cbor = rt.block_on(self.client.get_block_by_point(point))?;
        self.processor.process_block(&cbor)
    }

    async fn stream(
        &self,
        last_header: Option<BlockHeader>,
        min_batch_size: usize,
        _fetching_par: usize,
        processing_par: usize,
    ) -> Pin<Box<dyn Stream<Item = (Vec<Block<EuTx>>, BatchWeight)> + Send + 'life0>> {
        let last_point = last_header.clone().map_or(Point::Origin, |h| {
            Point::new(h.timestamp.0 as u64, h.hash.0.to_vec())
        });

        let (tx, rx) = mpsc::channel::<CBOR>(100);
        let node_client = Arc::clone(&self.client.node_client);

        tokio::spawn(async move {
            let (_, to) = node_client
                .lock()
                .await
                .chainsync()
                .find_intersect(vec![last_point])
                .await
                .unwrap();

            info!(
                "Indexing from {} to {}",
                last_header.map(|h| h.height.0).unwrap_or(0),
                to.1
            );
            loop {
                match node_client
                    .lock()
                    .await
                    .chainsync()
                    .request_or_await_next()
                    .await
                    .unwrap()
                {
                    NextResponse::RollForward(block_bytes, _) => {
                        if tx.send(block_bytes.0).await.is_err() {
                            break;
                        }
                    }
                    // Since we're just scraping data until we catch up, we don't need to handle rollbacks
                    NextResponse::RollBackward(_, _) => {}
                    // Await is returned once we've caught up, and we should let
                    // the node notify us when there's a new block available
                    NextResponse::Await => break,
                }
            }
        });

        ReceiverStream::new(rx)
            .map(|cbor| {
                let processor = Arc::clone(&self.processor);
                tokio::task::spawn_blocking(move || processor.process_block(&cbor).unwrap())
            })
            .buffered(processing_par)
            .map(|res| match res {
                Ok(block) => block,
                Err(e) => panic!("Error: {:?}", e),
            })
            .min_batch_with_weight(min_batch_size, |block| block.weight)
            .boxed()
    }
}
