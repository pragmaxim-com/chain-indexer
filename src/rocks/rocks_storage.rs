use crate::api::{CiBlock, Height};
use crate::rocks::rocks_io_indexer;
use crate::{api::Storage, rocks::rocks_io_indexer::RocksIoIndexer};
use broadcast_sink::Consumer;
use rocksdb::{MultiThreaded, Options, TransactionDB, TransactionDBOptions};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const ADDRESS_CF: &str = "ADDRESS_CF";
pub const CACHE_CF: &str = "CACHE_CF";
pub const META_CF: &str = "META_CF";

pub const LAST_ADDRESS_HEIGHT_KEY: &[u8] = b"last_address_height";

pub struct RocksStorage {
    db: Arc<TransactionDB<MultiThreaded>>,
}

impl RocksStorage {
    pub fn new(num_cores: i32, db_path: &str, cfs: Vec<&str>) -> Result<Self, String> {
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
            for cf in cfs.into_iter() {
                instance.create_cf(cf, &options).unwrap();
            }
        }

        Ok(RocksStorage {
            db: Arc::new(instance),
        })
    }
}

// implement BlockBatchIndexer trait
impl Storage for RocksStorage {
    fn get_last_height(&self) -> u64 {
        let db_clone = Arc::clone(&self.db);
        rocks_io_indexer::get_last_height(db_clone)
    }

    fn get_indexers(&self) -> Vec<Arc<Mutex<dyn Consumer<Vec<(Height, CiBlock)>>>>> {
        vec![Arc::new(Mutex::new(RocksIoIndexer::new(Arc::clone(
            &self.db,
        ))))]
    }
}
