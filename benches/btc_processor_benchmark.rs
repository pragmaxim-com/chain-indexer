use std::time::Duration;

use ci::{
    api::{BlockHeight, BlockProcessor, BlockTimestamp, BlockchainClient, TxCount},
    eutxo::btc::{btc_client::BtcClient, btc_processor::BtcProcessor},
    info, settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let config = settings::AppConfig::new().unwrap();
    let blockchain = config.blockchain;
    let api_host = blockchain.api_host;
    let api_username = blockchain.api_username;
    let api_password = blockchain.api_password;

    let btc_client = BtcClient::new(&api_host, &api_username, &api_password);
    let processor = BtcProcessor {};
    info!("Initiating download");
    let batch_size = 50000;
    let start_height = 1 as u32;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<(BlockHeight, bitcoin::Block, TxCount, BlockTimestamp)> =
        Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(btc_client.get_block(height).unwrap());
    }
    info!("Initiating processing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(1000));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("processor"), |bencher| {
        bencher.iter(|| {
            let xs = blocks.drain(0..10).collect();
            processor.process(&xs)
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
