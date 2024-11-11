use config::{Config, ConfigError, Environment, File};
use dotenv::dotenv;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Parallelism {
    Low,
    Mild,
    High,
}

impl FromStr for Parallelism {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Parallelism::Low),
            "mild" => Ok(Parallelism::Mild),
            "high" => Ok(Parallelism::High),
            _ => Err(format!("Invalid value for Parallelism: {}", s)),
        }
    }
}

impl Parallelism {
    pub fn to_numeric(&self) -> usize {
        match self {
            Parallelism::Low => num_cpus::get() / 8,
            Parallelism::Mild => num_cpus::get() / 4,
            Parallelism::High => num_cpus::get() / 2,
        }
    }
}

// Custom deserialization function for Parallelism to handle string values in the TOML file
impl<'de> serde::Deserialize<'de> for Parallelism {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Parallelism::from_str(&s).map_err(serde::de::Error::custom)
    }
}

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
    pub fetching_parallelism: Parallelism,
    pub processing_parallelism: Parallelism,
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
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        match dotenv() {
            Ok(_) => {
                let builder = Config::builder()
                    .add_source(File::with_name(path).required(true))
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
