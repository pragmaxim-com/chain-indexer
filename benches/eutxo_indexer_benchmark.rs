use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use ci::{
    api::{BlockHeight, BlockProcessor, BlockchainClient, Indexers},
    error,
    eutxo::{
        btc::{btc_client::BtcClient, btc_processor::BtcProcessor},
        eutxo_indexers::EutxoIndexers,
    },
    info, settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = settings::AppConfig::new().unwrap();
    let blockchain = config.blockchain;
    let api_host = blockchain.api_host;
    let api_username = blockchain.api_username;
    let api_password = blockchain.api_password;
    let db_path = format!("{}/{}/{}", blockchain.db_path, "benchmark", blockchain.name);
    let db_indexes = config.indexer.db_indexes;

    let btc_client = BtcClient::new(&api_host, &api_username, &api_password);
    let processor = BtcProcessor {};
    let storage = EutxoIndexers::new(&db_path, db_indexes);
    let indexers = storage.get_indexers();
    let indexer = indexers.get(0).unwrap();
    info!("Initiating download");
    let batch_size = 50000;
    let start_height = 1 as u32;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<(BlockHeight, bitcoin::Block, usize)> =
        Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(
            btc_client
                .get_block_with_tx_count_for_height(height)
                .unwrap(),
        );
    }
    info!("Initiating processing");
    let blocks = Arc::new(Mutex::new(processor.process(&blocks)));

    info!("Initiating indexing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(50));
    group.measurement_time(Duration::from_millis(500));
    group.bench_function(BenchmarkId::from_parameter("processor"), |bencher| {
        bencher.to_async(&rt).iter(|| async {
            let arc = Arc::clone(&blocks);
            let mut blocks_chunk = arc.lock().unwrap();
            let xs = blocks_chunk[0..10].to_vec();
            blocks_chunk.drain(0..10);
            if let Err(e) = indexer.lock().await.consume(&xs) {
                error!("BroadcastSink consumer error occurred: {:?}", e);
            }
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
