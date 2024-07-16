use crate::api::BlockchainClient;
use crate::model::{Block, BlockHash, BlockHeader, BlockHeight, TxCount};
use bitcoin_hashes::Hash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BtcBlock {
    pub height: BlockHeight, // expensive to calculate
    pub delegate: bitcoin::Block,
}

impl Block for BtcBlock {
    fn header(&self) -> BlockHeader {
        BlockHeader {
            height: self.height,
            timestamp: (self.delegate.header.time as i64).into(),
            hash: self.delegate.block_hash().to_byte_array().into(),
            parent_hash: self.delegate.header.prev_blockhash.to_byte_array().into(),
        }
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
                    height: (height as u32).into(),
                    delegate: block,
                })
            })
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> Result<BtcBlock, String> {
        let bitcoin_hash = bitcoin::BlockHash::from_slice(&hash.0).unwrap();
        let block = self
            .rpc_client
            .get_block(&bitcoin_hash)
            .map_err(|e| e.to_string())?;
        let height = block.bip34_block_height().map_err(|e| e.to_string())?;
        Ok(BtcBlock {
            height: (height as u32).into(),
            delegate: block,
        })
    }

    fn get_block_by_height(&self, height: BlockHeight) -> Result<BtcBlock, String> {
        self.rpc_client
            .get_block_hash(height.0 as u64)
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
