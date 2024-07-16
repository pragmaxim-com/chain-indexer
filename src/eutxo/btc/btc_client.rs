use crate::api::BlockchainClient;
use crate::model::{Block, BlockHash, BlockHeader, BlockHeight};
use bitcoin_hashes::Hash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BtcTx {
    pub height: BlockHeight, // expensive to calculate
    pub delegate: bitcoin::Block,
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

impl TryFrom<bitcoin::Block> for Block<bitcoin::Transaction> {
    type Error = String;

    fn try_from(block: bitcoin::Block) -> Result<Self, String> {
        let height = block.bip34_block_height().map_err(|e| e.to_string())?;
        let header = BlockHeader {
            height: (height as u32).into(),
            timestamp: (block.header.time as i64).into(),
            hash: block.block_hash().to_byte_array().into(),
            parent_hash: block.header.prev_blockhash.to_byte_array().into(),
        };
        Ok(Block::new(header, block.txdata))
    }
}

impl BlockchainClient for BtcClient {
    type Tx = bitcoin::Transaction;

    fn get_best_block(&self) -> Result<Block<bitcoin::Transaction>, String> {
        self.rpc_client
            .get_best_block_hash()
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;

                block.try_into()
            })
    }

    fn get_block_by_hash(&self, hash: BlockHash) -> Result<Block<Self::Tx>, String> {
        let bitcoin_hash = bitcoin::BlockHash::from_slice(&hash.0).unwrap();
        let block = self
            .rpc_client
            .get_block(&bitcoin_hash)
            .map_err(|e| e.to_string())?;

        block.try_into()
    }

    fn get_block_by_height(&self, height: BlockHeight) -> Result<Block<Self::Tx>, String> {
        self.rpc_client
            .get_block_hash(height.0 as u64)
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;
                // for performance reasons, we don't call existing From implementation due to hight calculation
                let header = BlockHeader {
                    height,
                    timestamp: (block.header.time as i64).into(),
                    hash: block.block_hash().to_byte_array().into(),
                    parent_hash: block.header.prev_blockhash.to_byte_array().into(),
                };
                Ok(Block::new(header, block.txdata))
            })
    }
}
