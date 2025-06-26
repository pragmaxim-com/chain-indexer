use std::sync::Arc;

use futures::lock::Mutex;
use pallas::network::{
    facades::{NodeClient, PeerClient},
    miniprotocols::{localstate::queries_v16, Point, MAINNET_MAGIC},
};

use crate::{api::ServiceError, info, settings::CardanoConfig};

pub type CBOR = Vec<u8>;

pub struct CardanoClient {
    pub peer_client: Arc<Mutex<PeerClient>>,
    pub node_client: Arc<Mutex<NodeClient>>,
}

impl CardanoClient {
    pub async fn new(cardano_config: &CardanoConfig) -> Self {
        let peer_client = Arc::new(Mutex::new(
            PeerClient::connect(cardano_config.api_host.clone(), MAINNET_MAGIC)
                .await
                .expect("Failed to connect to Cardano peer client"),
        ));
        let node_client = Arc::new(Mutex::new(
            NodeClient::connect(cardano_config.socket_path.clone(), MAINNET_MAGIC)
                .await
                .expect("Failed to connect to Cardano node client"),
        ));
        CardanoClient {
            peer_client,
            node_client,
        }
    }
}

impl CardanoClient {
    pub async fn get_best_block(&self) -> Result<CBOR, ServiceError> {
        let mut client = self.node_client.lock().await;
        info!("Getting chain tip from Cardano node client");
        let c = client.statequery();
        c.acquire(None).await?;
        let tip = queries_v16::get_chain_point(c).await?;
        c.send_release().await?;
        self.get_block_by_point(tip).await
    }

    pub async fn get_block_by_point(&self, point: Point) -> Result<CBOR, ServiceError> {
        info!("Getting block from Cardano peer client");
        let block = self
            .peer_client
            .lock()
            .await
            .blockfetch()
            .fetch_single(point)
            .await?;
        Ok(block)
    }
}
