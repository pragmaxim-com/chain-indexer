use backend::cli::{Blockchain, CliConfig};
use backend::eutxo::cardano::cardano_block_provider::CardanoBlockProvider;
use backend::eutxo::ergo::ergo_block_provider::ErgoBlockProvider;
use backend::eutxo::eutxo_executor;
use backend::eutxo::eutxo_schema::DbSchema;
use clap::Parser;
use std::sync::Arc;

use backend::eutxo::btc::btc_block_provider::BtcBlockProvider;
use backend::settings::AppConfig;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new("config/settings");
    let cli_clonfig: CliConfig = CliConfig::parse();
    let schema = DbSchema::load_config("config/schema.yaml");
    match config {
        Ok(app_config) => match cli_clonfig.blockchain {
            Blockchain::Bitcoin => {
                let block_provider =
                    Arc::new(BtcBlockProvider::new(&app_config.bitcoin, schema.bitcoin));
                eutxo_executor::run_eutxo_indexing_and_http_server(
                    app_config.indexer,
                    app_config.http,
                    cli_clonfig,
                    block_provider,
                )
                .await;
                Ok(())
            }
            Blockchain::Cardano => {
                let block_provider =
                    Arc::new(CardanoBlockProvider::new(&app_config.cardano, schema.cardano).await);
                eutxo_executor::run_eutxo_indexing_and_http_server(
                    app_config.indexer,
                    app_config.http,
                    cli_clonfig,
                    block_provider,
                )
                .await;
                Ok(())
            }
            Blockchain::Ergo => {
                let block_provider =
                    Arc::new(ErgoBlockProvider::new(&app_config.ergo, schema.ergo));
                eutxo_executor::run_eutxo_indexing_and_http_server(
                    app_config.indexer,
                    app_config.http,
                    cli_clonfig,
                    block_provider,
                )
                .await;
                Ok(())
            }
        },
        Err(e) => {
            backend::error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
