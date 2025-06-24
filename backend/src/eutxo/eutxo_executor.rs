use crate::cli::Blockchain;
use crate::cli::CliConfig;
use crate::http_server;
use crate::indexer::Indexer;
use crate::settings::HttpSettings;
use crate::settings::IndexerSettings;

use std::io;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::eutxo::eutxo_block_monitor::EuBlockMonitor;
use crate::info;
use crate::syncer::ChainSyncer;
use crate::{api::BlockProvider};
use actix_web::dev::Server;
use futures::future::join;
use futures::future::ready;
use redb::Database;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio::time;

async fn maybe_run_server(http_conf: &HttpSettings, server: Server) -> io::Result<()> {
    if http_conf.enable {
        info!("Starting http server at {}", http_conf.bind_address);
        server.await
    } else {
        ready(Ok(())).await
    }
}

pub async fn run_eutxo_indexing_and_http_server(
    indexer_conf: IndexerSettings,
    http_conf: HttpSettings,
    cli_config: CliConfig,
    indexer: Indexer,
    block_provider: Arc<dyn BlockProvider>,
    db: Arc<Database>
) {
    let perist_coinbase_inputs: bool = match cli_config.blockchain {
        Blockchain::Bitcoin => false,
        Blockchain::Cardano => true,
        Blockchain::Ergo => false,
    };

    let min_batch_size = indexer_conf.min_batch_size;
    let syncer = ChainSyncer::new(block_provider, Rc::new(EuBlockMonitor::new(1000)), indexer);

    let server = http_server::run(http_conf.clone(), Arc::clone(&db));

    let server_handle = server.handle();

    let server_fut = maybe_run_server(&http_conf, server);

    let indexing_fut = if indexer_conf.enable {
        async {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                syncer
                    .sync(
                        min_batch_size,
                        indexer_conf.fetching_parallelism.to_numeric(),
                        indexer_conf.processing_parallelism.to_numeric(),
                    )
                    .await;
                interval.tick().await;
            }
        }
        .await
    } else {
        ready(()) // Return an empty future if the feature flag is false
    };

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to listen for SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");

    let combined_fut = join(server_fut, indexing_fut);

    tokio::select! {
        _ = sigint.recv() => {
            println!("Received SIGINT, shutting down...");
        }
        _ = sigterm.recv() => {
            println!("Received SIGTERM, shutting down...");
        }
        _ = combined_fut => {

        }
    }

    info!("Stopping server.");
    server_handle.stop(true).await;
}
