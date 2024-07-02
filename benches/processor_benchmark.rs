use ci::{
    api::{BlockHeight, BlockProcessor, BlockchainClient},
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
    let batch_size = 10;
    let start_height = 500_000 as u32;
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
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.bench_with_input(
        BenchmarkId::from_parameter("processor"),
        &blocks,
        |bencher, blocks| {
            bencher.iter(|| processor.process(&blocks));
        },
    );
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
