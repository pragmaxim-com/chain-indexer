use crate::api::{Block, BlockHeight, BlockTimestamp, BlockchainClient, TxCount};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BtcBlock {
    pub height: BlockHeight, // expensive to calculate
    pub delegate: bitcoin::Block,
}

impl Block for BtcBlock {
    fn height(&self) -> BlockHeight {
        self.height
    }

    fn timestamp(&self) -> BlockTimestamp {
        self.delegate.header.time as i64
    }

    fn tx_count(&self) -> TxCount {
        self.delegate.txdata.len()
    }
}

pub struct BtcClient {
    rpc_client: Arc<Client>,
}

impl BtcClient {
    pub fn new(api_host: &str, api_username: &str, api_password: &str) -> Self {
        let user_pass = Auth::UserPass(api_username.to_string(), api_password.to_string());
        let client = Client::new(api_host, user_pass).unwrap();
        let rpc_client = Arc::new(client);
        BtcClient { rpc_client }
    }
}

impl BlockchainClient for BtcClient {
    type Block = BtcBlock;

    fn get_best_block(&self) -> Result<BtcBlock, String> {
        self.rpc_client
            .get_best_block_hash()
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;
                let height = block.bip34_block_height().map_err(|e| e.to_string())?;
                Ok(BtcBlock {
                    height: height as u32,
                    delegate: block,
                })
            })
    }

    fn get_block(&self, height: u32) -> Result<BtcBlock, String> {
        self.rpc_client
            .get_block_hash(height as u64)
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;
                Ok(BtcBlock {
                    height,
                    delegate: block,
                })
            })
    }
}
