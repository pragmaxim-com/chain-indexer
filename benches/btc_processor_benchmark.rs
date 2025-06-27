use std::{env, fs};
use std::sync::Arc;
use std::time::Duration;

use backend::api::BlockProcessor;
use backend::eutxo::btc::btc_client::BtcBlock;
use backend::eutxo::btc::btc_io_processor::BtcIoProcessor;
use backend::eutxo::eutxo_model::BlockHeight;
use backend::{
    eutxo::btc::{btc_block_processor::BtcBlockProcessor, btc_client::BtcClient},
    info,
    settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use backend::eutxo::eutxo_storage;

fn criterion_benchmark(c: &mut Criterion) {
    let config = settings::AppConfig::new("config/settings").unwrap();
    let btc_config = config.bitcoin;

    let db_name = format!("{}/{}", "btc_processor", "benchmark");
    let db_path = env::temp_dir().join(&db_name);
    fs::remove_dir_all(&db_path).unwrap();
    let db = Arc::new(eutxo_storage::get_db(db_path).expect("Failed to open database"));

    let btc_client = BtcClient::new(&btc_config);
    let processor = BtcBlockProcessor::new(BtcIoProcessor{});
    info!("Initiating download");
    let batch_size = 50000;
    let start_height = 1;
    let end_height = start_height + batch_size;
    let mut blocks: Vec<BtcBlock> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        blocks.push(btc_client.get_block_by_height(BlockHeight(height)).unwrap());
    }
    info!("Initiating processing");
    let mut group = c.benchmark_group("processor");
    group.throughput(Throughput::Elements(batch_size as u64));
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_millis(1000));
    group.bench_function(BenchmarkId::from_parameter("processor"), |bencher| {
        bencher.iter(|| {
            let read_tx = db.begin_read().expect("Failed to begin read transaction");
            blocks.pop().iter().for_each(|b| { 
                processor.process_block(&b, &read_tx).expect("Failed to process block");
                ()
            })
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
