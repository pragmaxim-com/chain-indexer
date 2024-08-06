use crate::model::{Block, BlockHash, BlockHeader, BlockHeight};
use ergo_lib::chain::block::FullBlock;
use ergo_lib::chain::transaction::Transaction;
use reqwest::{
    blocking,
    header::{HeaderValue, CONTENT_TYPE},
    Client, RequestBuilder, Url,
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

    fn set_async_req_headers(&self, rb: RequestBuilder) -> RequestBuilder {
        rb.header("accept", "application/json")
            .header("api_key", self.get_node_api_header())
            .header(CONTENT_TYPE, "application/json")
    }

    fn build_async_client(&self) -> Result<Client, String> {
        let builder = Client::builder();
        builder
            .timeout(Duration::from_millis(3000))
            .build()
            .map_err(|e| e.to_string())
    }

    fn set_blocking_req_headers(&self, rb: blocking::RequestBuilder) -> blocking::RequestBuilder {
        rb.header("accept", "application/json")
            .header("api_key", self.get_node_api_header())
            .header(CONTENT_TYPE, "application/json")
    }

    fn build_blocking_client(&self) -> Result<blocking::Client, String> {
        let builder = blocking::Client::builder();
        builder
            .timeout(Duration::from_millis(3000))
            .build()
            .map_err(|e| e.to_string())
    }

    pub(crate) async fn get_best_block_async(&self) -> Result<Block<Transaction>, String> {
        let url = self.node_url.join("info").unwrap();
        let rb = self.build_async_client()?.get(url);
        let node_info = self
            .set_async_req_headers(rb)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<NodeInfo>()
            .await
            .map_err(|e| e.to_string())?;

        self.get_block_by_height_async(node_info.full_height.into())
            .await
    }

    pub(crate) fn get_best_block_sync(&self) -> Result<Block<Transaction>, String> {
        let url = self.node_url.join("info").unwrap();
        let rb = self.build_blocking_client()?.get(url);
        let node_info = self
            .set_blocking_req_headers(rb)
            .send()
            .map_err(|e| e.to_string())?
            .json::<NodeInfo>()
            .map_err(|e| e.to_string())?;

        self.get_block_by_height_sync(node_info.full_height.into())
    }

    pub(crate) async fn get_block_by_height_async(
        &self,
        height: BlockHeight,
    ) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/at/".to_owned();
        path.push_str(&height.0.to_string());
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url_async(url).await
    }

    pub(crate) fn get_block_by_height_sync(
        &self,
        height: BlockHeight,
    ) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/at/".to_owned();
        path.push_str(&height.0.to_string());
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url_sync(url)
    }

    fn get_block_by_url_sync(&self, url: Url) -> Result<Block<Transaction>, String> {
        let rb = self.build_blocking_client()?.get(url);
        let block = self
            .set_blocking_req_headers(rb)
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

    async fn get_block_by_url_async(&self, url: Url) -> Result<Block<Transaction>, String> {
        let rb = self.build_async_client()?.get(url);
        let block = self
            .set_async_req_headers(rb)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<FullBlock>()
            .await
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

    pub(crate) fn get_block_by_hash_sync(
        &self,
        hash: BlockHash,
    ) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/".to_owned();
        let block_hash: String = hex::encode(hash.0);
        path.push_str(&block_hash);
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url_sync(url)
    }
    pub(crate) async fn get_block_by_hash_async(
        &self,
        hash: BlockHash,
    ) -> Result<Block<Transaction>, String> {
        let mut path = "blocks/".to_owned();
        let block_hash: String = hex::encode(hash.0);
        path.push_str(&block_hash);
        #[allow(clippy::unwrap_used)]
        let url = self.node_url.join(&path).unwrap();
        self.get_block_by_url_async(url).await
    }
}
