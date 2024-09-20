use crate::{api::ServiceError, error};
use ergo_lib::chain::block::FullBlock;
use model::{BlockHash, BlockHeight};
use reqwest::{
    blocking,
    header::{ACCEPT, CONTENT_TYPE},
    Client, RequestBuilder, Url,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
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
    pub(crate) api_key: String,
}

impl ErgoClient {
    fn set_async_req_headers(rb: RequestBuilder, api_key: &str) -> RequestBuilder {
        rb.header(ACCEPT, "application/json")
            .header("api_key", api_key)
            .header(CONTENT_TYPE, "application/json")
    }

    fn build_async_client() -> Result<Client, ServiceError> {
        let builder = Client::builder();
        let client = builder.timeout(Duration::from_millis(10000)).build()?;
        Ok(client)
    }

    fn set_blocking_req_headers(
        rb: blocking::RequestBuilder,
        api_key: &str,
    ) -> blocking::RequestBuilder {
        rb.header(ACCEPT, "application/json")
            .header("api_key", api_key)
            .header(CONTENT_TYPE, "application/json")
    }

    fn build_blocking_client() -> Result<blocking::Client, ServiceError> {
        let builder = blocking::Client::builder();
        let client = builder.timeout(Duration::from_millis(10000)).build()?;
        Ok(client)
    }

    pub(crate) async fn get_block_by_height_async(
        &self,
        height: BlockHeight,
    ) -> Result<FullBlock, ServiceError> {
        let block_ids = self.get_block_ids_by_height_async(height).await?;

        self.get_block_by_hash_async(block_ids.first().unwrap())
            .await
    }

    pub(crate) async fn get_best_block_async(&self) -> Result<FullBlock, ServiceError> {
        let node_info_url: Url = self.node_url.join("info")?;

        let response = ErgoClient::set_async_req_headers(
            ErgoClient::build_async_client()?.get(node_info_url),
            &self.api_key,
        )
        .send()
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| String::new());
            error!("Request failed with status {}: {}", status, text);
            Err(ServiceError::new(&"Request failed".to_string()))
        } else {
            let node_info = response.json::<NodeInfo>().await?;

            self.get_block_by_height_async(node_info.full_height.into())
                .await
        }
    }

    pub async fn get_block_ids_by_height_async(
        &self,
        height: BlockHeight,
    ) -> Result<Vec<String>, ServiceError> {
        let block_ids_url = self
            .node_url
            .join(&format!("blocks/at/{}", &height.0.to_string()))?;
        let block_ids = ErgoClient::set_async_req_headers(
            ErgoClient::build_async_client()?.get(block_ids_url),
            &self.api_key,
        )
        .send()
        .await?
        .json::<Vec<String>>()
        .await?;
        Ok(block_ids)
    }

    pub fn get_block_ids_by_height_sync(
        &self,
        height: BlockHeight,
    ) -> Result<Vec<String>, ServiceError> {
        let block_ids_url = self
            .node_url
            .join(&format!("blocks/at/{}", &height.0.to_string()))?;
        let block_ids = ErgoClient::set_blocking_req_headers(
            ErgoClient::build_blocking_client()?.get(block_ids_url),
            &self.api_key,
        )
        .send()?
        .json::<Vec<String>>()?;
        Ok(block_ids)
    }

    pub fn get_block_by_hash_sync(&self, hash: BlockHash) -> Result<FullBlock, ServiceError> {
        let url = self
            .node_url
            .join(&format!("blocks/{}", hex::encode(hash.0)))?;
        let block = ErgoClient::set_blocking_req_headers(
            ErgoClient::build_blocking_client()?.get(url),
            &self.api_key,
        )
        .send()?
        .json::<FullBlock>()?;
        Ok(block)
    }

    pub(crate) async fn get_block_by_hash_async(
        &self,
        block_hash: &str,
    ) -> Result<FullBlock, ServiceError> {
        let url = self.node_url.join(&format!("blocks/{}", block_hash))?;
        let block = ErgoClient::set_async_req_headers(
            ErgoClient::build_async_client()?.get(url),
            &self.api_key,
        )
        .send()
        .await?
        .json::<FullBlock>()
        .await?;
        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json;

    #[tokio::test]
    async fn test_info_request() {
        let url = Url::from_str("http://127.0.0.1:9053")
            .unwrap()
            .join("info")
            .unwrap();
        let rb = ErgoClient::build_async_client().unwrap().get(url);
        let response = ErgoClient::set_async_req_headers(rb, "")
            .send()
            .await
            .unwrap()
            .json::<NodeInfo>()
            .await
            .unwrap();
        println!("name: {}", response.name);
    }

    #[tokio::test]
    async fn test_block_ids_request() {
        let url = Url::from_str("http://127.0.0.1:9053")
            .unwrap()
            .join("blocks/at/100")
            .unwrap();

        let rb = ErgoClient::build_async_client().unwrap().get(url);
        let block_ids = ErgoClient::set_async_req_headers(rb, "")
            .send()
            .await
            .unwrap()
            .json::<Vec<String>>()
            .await
            .unwrap();
        println!("hash: {}", block_ids.first().unwrap());
    }

    #[test]
    fn test_deserialization() {
        let json_data = r#"
        {
          "currentTime" : 1723784804691,
          "network" : "mainnet",
          "name" : "ergo-mainnet-5.0.22",
          "stateType" : "utxo",
          "difficulty" : 1115063704354816,
          "bestFullHeaderId" : "db5095ab785ea515ec2fc76e1d890bec4d88318c118d9561fb4bb7f6069fbecb",
          "bestHeaderId" : "db5095ab785ea515ec2fc76e1d890bec4d88318c118d9561fb4bb7f6069fbecb",
          "peersCount" : 30,
          "unconfirmedCount" : 4,
          "appVersion" : "5.0.22",
          "eip37Supported" : true,
          "stateRoot" : "15dc211165746cc0625ae9c62ad8f4309c8983b36279a349207e09099beb857619",
          "genesisBlockId" : "b0244dfc267baca974a4caee06120321562784303a8a688976ae56170e4d175b",
          "previousFullHeaderId" : "c45b1984c7ed6e77c7955c22fa074e657dec7bb7141b5044f1d3b5c273c26897",
          "fullHeight" : 1331111,
          "headersHeight" : 1331111,
          "stateVersion" : "db5095ab785ea515ec2fc76e1d890bec4d88318c118d9561fb4bb7f6069fbecb",
          "fullBlocksScore" : 2396498399696617734144,
          "maxPeerHeight" : 1331111,
          "launchTime" : 1723784167386,
          "isExplorer" : false,
          "lastSeenMessageTime" : 1723784781950,
          "eip27Supported" : true,
          "headersScore" : 2396498399696617734144,
          "parameters" : {
            "outputCost" : 214,
            "tokenAccessCost" : 100,
            "maxBlockCost" : 8001091,
            "height" : 1330176,
            "maxBlockSize" : 1271009,
            "dataInputCost" : 100,
            "blockVersion" : 3,
            "inputCost" : 2407,
            "storageFeeFactor" : 1250000,
            "minValuePerByte" : 360
          },
          "isMining" : false
        }"#;

        // Deserialize the JSON data
        let node_info: NodeInfo = serde_json::from_str(json_data).expect("Failed to deserialize");

        // Expected NodeInfo struct
        let expected_node_info = NodeInfo {
            name: "ergo-mainnet-5.0.22".to_string(),
            app_version: "5.0.22".to_string(),
            full_height: 1331111,
        };

        // Assert that the deserialized data matches the expected struct
        assert_eq!(node_info, expected_node_info);
    }
}
