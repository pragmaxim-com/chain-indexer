use crate::model::{Block, BlockHash, BlockHeader, BlockHeight};
use ergo_lib::chain::block::FullBlock;
use ergo_lib::chain::transaction::Transaction;
use reqwest::{
    blocking::{Client, RequestBuilder},
    header::{HeaderValue, CONTENT_TYPE},
    Url,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[repr(C)]
pub struct NodeInfo {
    pub name: String,
    #[serde(rename = "appVersion")]
    pub app_version: String,
    #[serde(rename = "fullHeight")]
    pub full_height: u32,
}

pub struct ErgoClient {
    pub(crate) node_url: Url,
    pub(crate) api_key: Option<&'static str>,
}

impl ErgoClient {
    pub fn get_node_api_header(&self) -> HeaderValue {
        match self.api_key {
            Some(api_key) => match HeaderValue::from_str(api_key) {
                Ok(k) => k,
                _ => HeaderValue::from_static("None"),
            },
            None => HeaderValue::from_static("None"),
        }
    }

    fn set_req_headers(&self, rb: RequestBuilder) -> RequestBuilder {
        rb.header("accept", "application/json")
            .header("api_key", self.get_node_api_header())
            .header(CONTENT_TYPE, "application/json")
    }
}

impl ErgoClient {
    fn build_client(&self) -> Result<Client, String> {
        let builder = Client::builder();
        builder
            .timeout(Duration::from_millis(3000))
            .build()
            .map_err(|e| e.to_string())
    }

    pub(crate) fn get_best_block(&self) -> Result<Block<Transaction>, String> {
        let url = self.node_url.join("info").unwrap();
        let rb = self.build_client()?.get(url);
        let node_info = self
            .set_req_headers(rb)
            .send()
            .map_err(|e| e.to_string())?
            .json::<NodeInfo>()
            .map_err(|e| e.to_string())?;

        self.get_block_by_height(node_info.full_height.into())
    }

    pub(crate) fn get_block_by_height(
        &self,
        height: BlockHeight,
    ) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/at/".to_owned();
        path.push_str(&height.0.to_string());
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url(url)
    }

    fn get_block_by_url(&self, url: Url) -> Result<Block<Transaction>, String> {
        let rb = self.build_client()?.get(url);
        let block = self
            .set_req_headers(rb)
            .send()
            .map_err(|e| e.to_string())?
            .json::<FullBlock>()
            .map_err(|e| e.to_string())?;
        let block_hash: [u8; 32] = block.header.id.0.into();
        let prev_block_hash: [u8; 32] = block.header.parent_id.0.into();
        let header = BlockHeader {
            height: block.header.height.into(),
            timestamp: (block.header.timestamp as u32).into(),
            hash: block_hash.into(),
            prev_hash: prev_block_hash.into(),
        };
        Ok(Block::new(
            header,
            block.block_transactions.transactions.to_vec(),
        ))
    }

    pub(crate) fn get_block_by_hash(&self, hash: BlockHash) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/".to_owned();
        let block_hash: String = hex::encode(hash.0);
        path.push_str(&block_hash);
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url(url)
    }
}
