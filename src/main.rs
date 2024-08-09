use ci::eutxo::cardano::cardano_block_provider::CardanoBlockProvider;
use ci::eutxo::eutxo_executor;

use std::sync::Arc;

use ci::eutxo::btc::btc_block_provider::BtcBlockProvider;
use ci::settings::AppConfig;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new();

    match config {
        Ok(config) => match config.blockchain.name.as_str() {
            "btc" => {
                let api_host = &config.blockchain.api_host;
                let api_username = &config.blockchain.api_username;
                let api_password = &config.blockchain.api_password;
                let block_provider =
                    Arc::new(BtcBlockProvider::new(api_host, api_username, api_password));

                eutxo_executor::run_eutxo_indexing(config, block_provider).await;
                Ok(())
            }
            "cardano" => {
                let api_host = &config.blockchain.api_host;
                let socket_path = &config.blockchain.socket_path;
                let block_provider =
                    Arc::new(CardanoBlockProvider::new(api_host, socket_path).await);

                eutxo_executor::run_eutxo_indexing(config, block_provider).await;
                Ok(())
            }
            _ => {
                ci::error!("Unsupported blockchain: {}", config.blockchain.name);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"));
            }
        },
        Err(e) => {
            ci::error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
