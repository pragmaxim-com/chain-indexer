use crate::{api::ServiceError, settings::BitcoinConfig};
use bitcoin::Block;
use bitcoin_hashes::Hash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use model::{BlockHash, BlockHeight};
use std::sync::Arc;

pub struct BtcClient {
    rpc_client: Arc<Client>,
}

impl BtcClient {
    pub fn new(bitcoin_config: &BitcoinConfig) -> Self {
        let user_pass = Auth::UserPass(
            bitcoin_config.api_username.to_string(),
            bitcoin_config.api_password.to_string(),
        );
        let client = Client::new(bitcoin_config.api_host.as_str(), user_pass).unwrap();
        let rpc_client = Arc::new(client);
        BtcClient { rpc_client }
    }
}

impl BtcClient {
    pub fn get_best_block(&self) -> Result<Block, ServiceError> {
        let best_block = self
            .rpc_client
            .get_best_block_hash()
            .and_then(|hash| self.rpc_client.get_block(&hash))?;
        Ok(best_block)
    }

    pub fn get_block_by_hash(&self, hash: BlockHash) -> Result<Block, ServiceError> {
        let bitcoin_hash = bitcoin::BlockHash::from_slice(&hash.0).unwrap();
        let block = self.rpc_client.get_block(&bitcoin_hash)?;
        Ok(block)
    }

    pub fn get_block_by_height(&self, height: BlockHeight) -> Result<Block, ServiceError> {
        let block_hash = self
            .rpc_client
            .get_block_hash(height.0 as u64)
            .and_then(|hash| self.rpc_client.get_block(&hash))?;
        Ok(block_hash)
    }
}
