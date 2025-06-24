use std::env;
use backend::cli::{Blockchain, CliConfig};
use backend::eutxo::cardano::cardano_block_provider::CardanoBlockProvider;
use backend::eutxo::{eutxo_executor, eutxo_storage};
use clap::Parser;
use std::sync::Arc;
use backend::api::BlockProvider;
use backend::eutxo::btc::btc_block_provider::BtcBlockProvider;
use backend::eutxo::ergo::ergo_block_provider::ErgoBlockProvider;
use backend::indexer::Indexer;
use backend::settings::AppConfig;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = AppConfig::new("config/settings");
    let cli_config: CliConfig = CliConfig::parse();
    match config {
        Ok(app_config) => {
            let db_path: String = format!(
                "{}/{}/{}",
                app_config.indexer.db_path, "main", cli_config.blockchain
            );
            let db = Arc::new(eutxo_storage::get_db(env::home_dir().unwrap().join(&db_path)).expect("Failed to open database"));

            match cli_config.blockchain {
                Blockchain::Bitcoin => {
                    let block_provider: Arc<dyn BlockProvider> = Arc::new(BtcBlockProvider::new(&app_config.bitcoin, Arc::clone(&db)));
                    let indexer = Indexer::new(Arc::clone(&db), Arc::clone(&block_provider));
                    eutxo_executor::run_eutxo_indexing_and_http_server(
                        app_config.indexer,
                        app_config.http,
                        cli_config,
                        indexer,
                        Arc::clone(&block_provider),
                        Arc::clone(&db)
                    )
                        .await;
                    Ok(())
                }
                Blockchain::Cardano => {
                    let block_provider: Arc<dyn BlockProvider> = Arc::new(CardanoBlockProvider::new(&app_config.cardano, Arc::clone(&db)).await);
                    let indexer = Indexer::new(Arc::clone(&db), Arc::clone(&block_provider));
                    eutxo_executor::run_eutxo_indexing_and_http_server(
                        app_config.indexer,
                        app_config.http,
                        cli_config,
                        indexer,
                        Arc::clone(&block_provider),
                        Arc::clone(&db)
                    )
                        .await;
                    Ok(())
                }
                Blockchain::Ergo => {
                    let block_provider: Arc<dyn BlockProvider> =
                        Arc::new(ErgoBlockProvider::new(&app_config.ergo, Arc::clone(&db)));
                    let indexer = Indexer::new(Arc::clone(&db), Arc::clone(&block_provider));
                    eutxo_executor::run_eutxo_indexing_and_http_server(
                        app_config.indexer,
                        app_config.http,
                        cli_config,
                        indexer,
                        Arc::clone(&block_provider),
                        Arc::clone(&db)
                    )
                        .await;
                    Ok(())
                }
            }
        },
        Err(e) => {
            backend::error!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Error"))
        }
    }
}
