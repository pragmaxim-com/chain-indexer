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

fn get_block_with_tx_count(
    rpc_client: Arc<Client>,
    hash: &bitcoin::BlockHash,
) -> Result<(bitcoin::Block, usize), String> {
    let block = rpc_client.get_block(hash).map_err(|e| e.to_string())?;
    let tx_count = block.txdata.len();
    Ok((block, tx_count))
}

impl BlockchainClient for BtcClient {
    type Block = bitcoin::Block;

    fn get_block_with_tx_count_for_height(
        &self,
        height: u32,
    ) -> Result<(bitcoin::Block, usize), String> {
        self.rpc_client
            .get_block_hash(height as u64)
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                get_block_with_tx_count(Arc::clone(&self.rpc_client), &hash)
                    .map_err(|e| e.to_string())
            })
    }
}
