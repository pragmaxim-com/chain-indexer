use config::{Config, ConfigError, Environment, File};
use dotenv::dotenv;
use serde::Deserialize;

use crate::{
    eutxo::{
        btc::btc_config::BitcoinConfig, cardano::cardano_config::CardanoConfig,
        ergo::ergo_config::ErgoConfig,
    },
    model::{DbIndexCfIndex, DbIndexUtxoBirthPkWithUtxoPkCf, DbIndexValue},
};

pub trait Indexes<I> {
    fn get_indexes(&self) -> Vec<DbIndexUtxoBirthPkWithUtxoPkCf>;
    fn create_indexes(&self, indexes: I) -> Vec<(DbIndexCfIndex, DbIndexValue)>;
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub indexer: IndexerSettings,
    pub bitcoin: BitcoinConfig,
    pub cardano: CardanoConfig,
    pub ergo: ErgoConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexerSettings {
    pub db_path: String,
    pub tx_batch_size: usize,
    pub disable_wal: bool,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        match dotenv() {
            Ok(_) => {
                let builder = Config::builder()
                    .add_source(File::with_name("config/settings").required(true))
                    .add_source(File::with_name("local-settings").required(false))
                    .add_source(
                        Environment::with_prefix("BITCOIN")
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
