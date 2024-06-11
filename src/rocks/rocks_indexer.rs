use crate::api::{BlockBatchIndexer, CiBlock, Height};
use crate::rocks::rocks_io_indexer;
use rocksdb::{MultiThreaded, Options, TransactionDB, TransactionDBOptions};
use std::sync::Arc;
use tokio::sync::broadcast::error::SendError;
use tokio::sync::{broadcast, Barrier};
use tokio::task::{self, JoinHandle};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub const ADDRESS_CF: &str = "ADDRESS_CF";
pub const CACHE_CF: &str = "CACHE_CF";
pub const META_CF: &str = "META_CF";

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

const CHANNEL_CAPACITY: usize = 100;

pub struct RocksIndexer {
    db: Arc<TransactionDB<MultiThreaded>>,
    tx: broadcast::Sender<Arc<Vec<(u64, CiBlock)>>>,
}

impl RocksIndexer {
    pub fn new(
        num_cores: i32,
        db_path: &str,
        cfs: Vec<&str>,
    ) -> Result<(Vec<JoinHandle<()>>, Self), String> {
        if cfs.is_empty() {
            panic!("Column Families must be non-empty");
        }
        let mut opts = Options::default();
        opts.create_if_missing(true);
        // Increase parallelism: setting the number of background threads
        opts.increase_parallelism(num_cores / 2); // Set this based on your CPU cores
        opts.set_max_background_jobs(std::cmp::max(num_cores / 2, 6));
        // Set other options for performance
        opts.set_max_file_opening_threads(std::cmp::max(num_cores / 2, 6));
        opts.set_write_buffer_size(128 * 1024 * 1024); // 64 MB
        opts.set_max_write_buffer_number(8);
        opts.set_target_file_size_base(128 * 1024 * 1024); // 64 MB
        opts.set_max_bytes_for_level_base(512 * 1024 * 1024);
        opts.set_use_direct_io_for_flush_and_compaction(true);

        let existing_cfs =
            TransactionDB::<MultiThreaded>::list_cf(&opts, db_path).unwrap_or(vec![]);

        let txn_db_opts = TransactionDBOptions::default();

        let instance = TransactionDB::<MultiThreaded>::open_cf(
            &opts,
            &txn_db_opts,
            db_path.to_string(),
            &existing_cfs,
        )
        .unwrap();

        if existing_cfs.is_empty() {
            let options = rocksdb::Options::default();
            for cf in cfs.iter() {
                instance.create_cf(cf, &options).unwrap();
            }
        }
        let db = Arc::new(instance);

        // Create tokio broadcast channel
        let (tx, _rx) = broadcast::channel::<Arc<Vec<(u64, CiBlock)>>>(CHANNEL_CAPACITY);

        // Barrier to synchronize all consumers
        let cf_names = vec![1];
        let barrier = Arc::new(Barrier::new(cf_names.len()));

        // Spawn consumer tasks
        let consumers: Vec<JoinHandle<()>> = cf_names
            .into_iter()
            .map(|cf_name| {
                let barrier_clone = Arc::clone(&barrier);
                let rx = tx.subscribe();

                let db_clone = Arc::clone(&db);
                task::spawn(async move {
                    let mut stream = BroadcastStream::new(rx);
                    while let Some(Ok(blocks_clone)) = stream.next().await {
                        rocks_io_indexer::index_blocks(db_clone.clone(), &blocks_clone);
                        barrier_clone.wait().await;
                    }
                })
            })
            .collect();

        Ok((consumers, RocksIndexer { db, tx }))
    }
}

// implement BlockBatchIndexer trait
impl BlockBatchIndexer for RocksIndexer {
    fn get_last_height(&self) -> u64 {
        let db_clone = Arc::clone(&self.db);
        rocks_io_indexer::get_last_height(db_clone)
    }
    fn index(
        &self,
        block_batch: Arc<Vec<(Height, CiBlock)>>,
    ) -> Result<usize, SendError<Arc<Vec<(Height, CiBlock)>>>> {
        self.tx.send(block_batch)
    }
}
