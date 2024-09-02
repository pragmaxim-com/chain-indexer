use config::{Config, ConfigError, Environment, File};
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub indexer: IndexerSettings,
    pub http: HttpSettings,
    pub bitcoin: BitcoinConfig,
    pub cardano: CardanoConfig,
    pub ergo: ErgoConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexerSettings {
    pub enable: bool,
    pub db_path: String,
    pub tx_batch_size: usize,
    pub disable_wal: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpSettings {
    pub enable: bool,
    pub bind_address: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BitcoinConfig {
    pub api_host: String,
    pub api_username: String,
    pub api_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ErgoConfig {
    pub api_host: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CardanoConfig {
    pub api_host: String,
    pub socket_path: String,
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
                    )
                    .add_source(
                        Environment::with_prefix("ERGO")
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
