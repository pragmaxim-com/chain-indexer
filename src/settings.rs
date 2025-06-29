use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::net::SocketAddr;
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

impl From<Parallelism> for usize {
    fn from(parallelism: Parallelism) -> Self {
        match parallelism {
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexerSettings {
    pub enable: bool,
    pub db_path: String,
    pub min_batch_size: usize,
    pub fetching_parallelism: Parallelism,
    pub processing_parallelism: Parallelism,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpSettings {
    pub enable: bool,
    pub bind_address: SocketAddr,
}

impl AppConfig {
    pub fn new(path: &str) -> Result<Self, ConfigError> {
        let builder = Config::builder().add_source(File::with_name(path).required(true));
        let config = builder.build()?.try_deserialize();
        println!("{:#?}", config);
        config
    }
}
