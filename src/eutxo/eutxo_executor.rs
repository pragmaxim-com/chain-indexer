use crate::block_read_service::BlockReadService;
use crate::cli::Blockchain;
use crate::cli::CliConfig;
use crate::experiment::Persistence;
use crate::http::paths;
use crate::indexer::Indexer;
use crate::settings::HttpSettings;
use crate::settings::IndexerSettings;

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::block_write_service::BlockWriteService;
use crate::eutxo::eutxo_block_monitor::EuBlockMonitor;
use crate::eutxo::eutxo_model::EuTx;
use crate::info;
use crate::syncer::ChainSyncer;
use crate::{api::BlockProvider, eutxo::eutxo_storage};
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::time;

use super::eutxo_tx_read_service::EuTxReadService;
use super::eutxo_tx_write_service::EuTxWriteService;

pub async fn run_eutxo_indexing_and_http_server(
    indexer_conf: IndexerSettings,
    http_conf: HttpSettings,
    cli_config: CliConfig,
    block_provider: Arc<dyn BlockProvider<OutTx = EuTx>>,
) {
    let db_path: String = format!(
        "{}/{}/{}",
        indexer_conf.db_path, "main", cli_config.blockchain
    );
    let perist_coinbase_inputs: bool = match cli_config.blockchain {
        Blockchain::Bitcoin => false,
        Blockchain::Cardano => true,
        Blockchain::Ergo => false,
    };

    let disable_wal = indexer_conf.disable_wal;
    let tx_batch_size = indexer_conf.tx_batch_size;
    let db_shema = block_provider.get_schema();
    let db = Arc::new(eutxo_storage::get_db(&db_shema, &db_path));

    let storage = Arc::new(Persistence::new(Arc::clone(&db), &db_shema));

    let tx_read_service = Arc::new(EuTxReadService::new(Arc::clone(&storage)));

    let block_read_service = Arc::new(BlockReadService::new(Arc::clone(&storage), tx_read_service));
    let block_write_service = Arc::new(BlockWriteService::new(
        Arc::new(EuTxWriteService::new(perist_coinbase_inputs)),
        Arc::clone(&block_read_service),
    ));

    let indexer = Indexer::new(
        Arc::clone(&storage),
        block_write_service,
        Arc::clone(&block_provider),
        disable_wal,
    );
    let syncer = ChainSyncer::new(block_provider, Rc::new(EuBlockMonitor::new(1000)), indexer);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&block_read_service)))
            .route("/blocks", web::post().to(paths::blocks::get_block_by_hash))
    })
    .bind(http_conf.bind_address.clone())
    .unwrap()
    .run();

    let server_handle = server.handle();

    info!("Starting Indexing into {}", db_path);
    let indexing_fut = async {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            syncer.sync(tx_batch_size).await;
            interval.tick().await;
        }
    };

    info!("Starting http server at {}", http_conf.bind_address);
    let server_fut = async { server.await };

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to listen for SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");

    tokio::select! {
        _ = sigint.recv() => {
            println!("Received SIGINT, shutting down...");
        }
        _ = sigterm.recv() => {
            println!("Received SIGTERM, shutting down...");
        }
        _ = server_fut => {
            // This branch should not happen since server_fut runs indefinitely
        }
        _ = indexing_fut => {
            // This branch should not happen since indexing_fut runs indefinitely
        }
    }

    info!("Stopping server.");
    server_handle.stop(true).await;
}
