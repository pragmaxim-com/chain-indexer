use std::{fs, sync::Arc, time::Duration};

use ci::{
    api::{BlockProcessor, BlockProvider, BlockchainClient, Storage},
    block_service::BlockService,
    eutxo::{
        btc::{
            btc_block_provider::BtcBlockProvider, btc_client::BtcClient,
            btc_processor::BtcProcessor,
        },
        eutxo_index_manager::DbIndexManager,
        eutxo_model::EuTx,
        eutxo_storage,
        eutxo_tx_service::EuTxService,
    },
    indexer::Indexer,
    info,
    model::Block,
    settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let config = settings::AppConfig::new().unwrap();
    let blockchain = config.blockchain;
    let api_host = blockchain.api_host;
    let api_username = blockchain.api_username;
    let api_password = blockchain.api_password;
    let db_path = format!("{}/{}/{}", blockchain.db_path, "benchmark", blockchain.name);
    let db_indexes = config.indexer.db_indexes;

    fs::remove_dir_all(&db_path).unwrap();

    let btc_client = BtcClient::new(&api_host, &api_username, &api_password);
    let processor = BtcProcessor {};
    let db_index_manager = DbIndexManager::new(&db_indexes);
    let db = eutxo_storage::get_db(&db_index_manager, &db_path);
    let get_families = eutxo_storage::get_families(&db_index_manager, &db);
    let families = get_families;
    let storage = Storage {
        db: &db,
        families: &families,
    };
    let tx_service: Arc<EuTxService> = Arc::new(EuTxService {});
    let block_service = Arc::new(BlockService::new(tx_service));

    let block_provider: Arc<
        dyn BlockProvider<InTx = bitcoin::Transaction, OutTx = EuTx> + Send + Sync,
    > = Arc::new(BtcBlockProvider::new(
        &api_host,
        &api_username,
        &api_password,
    ));

    let indexer = Arc::new(Indexer::new(
        &storage,
        block_service,
        Arc::clone(&block_provider),
    ));
    info!("Initiating download");
    let batch_size = 100_000;
    let start_height = 1 as u32;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<Block<bitcoin::Transaction>> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(btc_client.get_block_by_height(height.into()).unwrap());
    }

    info!("Initiating processing");
    let mut blocks = processor.process_batch(&blocks, 0).0;

    info!("Initiating indexing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("indexing"), |bencher| {
        bencher.iter(|| {
            let xs = blocks.drain(0..10).collect();
            indexer.persist_blocks(xs, false)
        });
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
