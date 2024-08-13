use ci::cli::{Blockchain, CliConfig};
use ci::eutxo::cardano::cardano_block_provider::CardanoBlockProvider;
use ci::eutxo::eutxo_executor;
use clap::Parser;
use std::sync::Arc;

use ci::eutxo::btc::btc_block_provider::BtcBlockProvider;
use ci::settings::AppConfig;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new();
    let cli_clonfig: CliConfig = CliConfig::parse();

    match config {
        Ok(app_config) => match cli_clonfig.blockchain {
            Blockchain::Bitcoin => {
                let block_provider = Arc::new(BtcBlockProvider::new(&app_config.bitcoin));

                eutxo_executor::run_eutxo_indexing(app_config, cli_clonfig, block_provider).await;
                Ok(())
            }
            Blockchain::Cardano => {
                let block_provider = Arc::new(CardanoBlockProvider::new(&app_config.cardano).await);

                eutxo_executor::run_eutxo_indexing(app_config, cli_clonfig, block_provider).await;
                Ok(())
            }
        },
        Err(e) => {
            ci::error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
