use crate::model::{Block, BlockHash, BlockHeader, BlockHeight};
use bitcoin_hashes::Hash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::sync::Arc;

use super::btc_config::BitcoinConfig;

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

impl TryFrom<bitcoin::Block> for Block<bitcoin::Transaction> {
    type Error = String;

    fn try_from(block: bitcoin::Block) -> Result<Self, String> {
        let height = block.bip34_block_height().map_err(|e| e.to_string())?;
        let header = BlockHeader {
            height: (height as u32).into(),
            timestamp: block.header.time.into(),
            hash: block.block_hash().to_byte_array().into(),
            prev_hash: block.header.prev_blockhash.to_byte_array().into(),
        };
        Ok(Block::new(header, block.txdata))
    }
}

impl BtcClient {
    pub fn get_best_block(&self) -> Result<BlockHeader, String> {
        self.rpc_client
            .get_best_block_hash()
            .map_err(|e| e.to_string())
            .and_then(|hash| {
                let block = self
                    .rpc_client
                    .get_block(&hash)
                    .map_err(|e| e.to_string())?;

                let b: Block<bitcoin::Transaction> = block.try_into()?;
                Ok(b.header)
            })
    }

    pub fn get_block_by_hash(
        &self,
        hash: BlockHash,
    ) -> Result<Block<bitcoin::Transaction>, String> {
        let bitcoin_hash = bitcoin::BlockHash::from_slice(&hash.0).unwrap();
        let block = self
            .rpc_client
            .get_block(&bitcoin_hash)
            .map_err(|e| e.to_string())?;

        block.try_into()
    }

    pub fn get_block_by_height(
        &self,
        height: BlockHeight,
    ) -> Result<Block<bitcoin::Transaction>, String> {
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
                    timestamp: block.header.time.into(),
                    hash: block.block_hash().to_byte_array().into(),
                    prev_hash: block.header.prev_blockhash.to_byte_array().into(),
                };
                Ok(Block::new(header, block.txdata))
            })
    }
}
