use config::{Config, ConfigError, Environment, File};
use dotenv::dotenv;
use serde::Deserialize;

use crate::model::DbIndexName;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub blockchain: BlockchainSettings,
    pub indexer: IndexerSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockchainSettings {
    pub name: String,
    pub db_path: String,
    pub api_host: String,
    pub api_username: String,
    pub api_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexerSettings {
    pub db_indexes: Vec<DbIndexName>,
    pub tx_batch_size: usize,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        match dotenv() {
            Ok(_) => {
                let builder = Config::builder()
                    .add_source(File::with_name("config/settings").required(true))
                    .add_source(File::with_name("local-settings").required(false))
                    .add_source(
                        Environment::with_prefix("BLOCKCHAIN")
                            .try_parsing(true)
                            .keep_prefix(true)
                            .separator("__"),
                    );
                let config = builder.build()?.try_deserialize();
                println!("{:#?}", config);
                config
            }
            Err(_) => panic!("Error loading .env file"),
        }
    }
}
