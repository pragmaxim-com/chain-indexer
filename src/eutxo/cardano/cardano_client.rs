use std::sync::Arc;

use futures::lock::Mutex;
use pallas::network::{
    facades::{NodeClient, PeerClient},
    miniprotocols::{localstate::queries_v16, Point, MAINNET_MAGIC},
};

use crate::settings::CardanoConfig;

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
                .unwrap(),
        ));
        let node_client = Arc::new(Mutex::new(
            NodeClient::connect(cardano_config.socket_path.clone(), MAINNET_MAGIC)
                .await
                .unwrap(),
        ));
        CardanoClient {
            peer_client,
            node_client,
        }
    }
}

impl CardanoClient {
    pub async fn get_best_block(&self) -> Result<CBOR, String> {
        let mut client = self.node_client.lock().await;
        let tip = queries_v16::get_chain_point(client.statequery())
            .await
            .map_err(|e| e.to_string())?;
        self.get_block_by_point(tip).await
    }

    pub async fn get_block_by_point(&self, point: Point) -> Result<CBOR, String> {
        let block_str = self
            .peer_client
            .lock()
            .await
            .blockfetch()
            .fetch_single(point)
            .await
            .map_err(|e| e.to_string())?;
        hex::decode(&block_str).map_err(|e| e.to_string())
    }
}
