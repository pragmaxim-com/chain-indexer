use config::{Config, ConfigError, Environment, File};
use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub blockchain: BlockchainSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockchainSettings {
    pub name: String,
    pub db_path: String,
    pub api_host: String,
    pub api_username: String,
    pub api_password: String,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        match dotenv() {
            Ok(_) => {
                let builder = Config::builder()
                    .add_source(File::with_name("config/settings").required(false))
                    .add_source(
                        Environment::with_prefix("BLOCKCHAIN")
                            .try_parsing(true)
                            .keep_prefix(true)
                            .separator("."),
                    );
                let config = builder.build()?.try_deserialize();
                println!("{:#?}", config);
                config
            }
            Err(_) => panic!("Error loading .env file"),
        }
    }
}
