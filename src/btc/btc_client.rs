use crate::api::BlockchainClient;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

pub struct BtcClient {
    rpc_client: Arc<Client>,
}

impl BtcClient {
    pub fn new(api_host: &str, api_username: &str, api_password: &str) -> Self {
        let user_pass = Auth::UserPass(api_username.to_string(), api_password.to_string());
        let rpc_client = Arc::new(Client::new(api_host, user_pass).unwrap());
        BtcClient { rpc_client }
    }
}

impl BlockchainClient for BtcClient {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    fn get_block_with_tx_count_for_height(
        &self,
        height: u64,
    ) -> Result<(bitcoin::Block, usize), String> {
        self.rpc_client
            .get_block_hash(height)
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                self.get_block_with_tx_count(&hash)
                    .map_err(|e| e.to_string())
            })
    }

    fn get_block_hash(&self, height: u64) -> Result<bitcoin::BlockHash, String> {
        let block_hash = self
            .rpc_client
            .get_block_hash(height)
            .map_err(|e| e.to_string())?;
        Ok(block_hash)
    }

    fn get_block_with_tx_count(
        &self,
        hash: &bitcoin::BlockHash,
    ) -> Result<(bitcoin::Block, usize), String> {
        let block = self.rpc_client.get_block(hash).map_err(|e| e.to_string())?;
        let tx_count = block.txdata.len();
        Ok((block, tx_count))
    }
}
