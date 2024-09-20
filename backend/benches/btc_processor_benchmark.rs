use std::time::Duration;

use backend::{
    api::BlockProcessor,
    eutxo::btc::{btc_block_processor::BtcBlockProcessor, btc_client::BtcClient},
    info,
    model::Block,
    settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let config = settings::AppConfig::new("config/settings").unwrap();
    let blockchain = config.blockchain;
    let api_host = blockchain.api_host;
    let api_username = blockchain.api_username;
    let api_password = blockchain.api_password;

    let btc_client = BtcClient::new(&api_host, &api_username, &api_password);
    let processor = BtcBlockProcessor {};
    info!("Initiating download");
    let batch_size = 50000;
    let start_height = 1 as u32;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<Block<bitcoin::Transaction>> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(btc_client.get_block_by_height(height.into()).unwrap());
    }
    info!("Initiating processing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("processor"), |bencher| {
        bencher.iter(|| {
            let xs = blocks.drain(0..10).collect();
            processor.process_batch(&xs, 0)
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
