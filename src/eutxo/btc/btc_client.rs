use crate::api::{BlockHeight, BlockTimestamp, BlockchainClient, TxCount};
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

    fn get_block(
        &self,
        height: u32,
    ) -> Result<(BlockHeight, bitcoin::Block, TxCount, BlockTimestamp), String> {
        self.rpc_client
            .get_block_hash(height as u64)
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;
                let tx_count = block.txdata.len();
                let timestamp = block.header.time;
                Ok((height, block, tx_count, timestamp as i64))
            })
    }
}
