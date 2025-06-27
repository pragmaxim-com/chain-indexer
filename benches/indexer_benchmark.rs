use std::{env, fs, sync::Arc, time::Duration};

use backend::eutxo::btc::btc_client::BtcBlock;
use backend::eutxo::btc::btc_io_processor::BtcIoProcessor;
use backend::eutxo::eutxo_model::BlockHeight;
use backend::{
    api::{BlockProcessor, BlockProvider},
    eutxo::{
        btc::{
            btc_block_processor::BtcBlockProcessor, btc_block_provider::BtcBlockProvider,
            btc_client::BtcClient,
        },
        eutxo_storage,
    },
    indexer::Indexer,
    info,
    settings,
};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let config = settings::AppConfig::new("config/settings").unwrap();
    let btc_config = config.bitcoin;
    let db_name = format!("{}/{}", "btc_indexer", "benchmark");
    let db_path = env::temp_dir().join(&db_name);
    fs::remove_dir_all(&db_path).unwrap();
    let db = Arc::new(eutxo_storage::get_db(db_path).expect("Failed to open database"));

    let btc_client = BtcClient::new(&btc_config);
    let processor = BtcBlockProcessor::new(BtcIoProcessor{});

    let block_provider: Arc<dyn BlockProvider> = Arc::new(BtcBlockProvider::new(&btc_config, Arc::clone(&db)));
    let indexer = Arc::new(Indexer::new(Arc::clone(&db), Arc::clone(&block_provider)));

    info!("Initiating download");
    let batch_size = 100_000;
    let start_height = 1 as u32;
    let end_height = start_height + batch_size;
    let mut btc_blocks: Vec<BtcBlock> = Vec::with_capacity(batch_size as usize);
    for height in start_height..end_height {
        btc_blocks.push(btc_client.get_block_by_height(BlockHeight(height)).unwrap());
    }

    info!("Initiating processing");
    let mut blocks = Vec::with_capacity(btc_blocks.len());
    let read_tx = db.begin_read().expect("Failed to begin read transaction");
    for block in btc_blocks.iter() {
        let b = processor.process_block(block, &read_tx).expect("Failed to process block");
        blocks.push(b);
    }

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
