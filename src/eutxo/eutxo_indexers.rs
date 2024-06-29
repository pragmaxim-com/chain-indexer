use crate::eutxo::eutxo_api::CiBlock;
use crate::log;
use crate::{api::Indexers, eutxo};
use broadcast_sink::Consumer;
use rocksdb::{MultiThreaded, Options, TransactionDB, TransactionDBOptions};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct EutxoIndexers {
    db: Arc<TransactionDB<MultiThreaded>>,
}

impl EutxoIndexers {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let num_cores = num_cpus::get() as i32;
        log!("Number of CPU cores: {}", num_cores);

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
            for cf in eutxo::eutxo_input_indexer::get_column_families().into_iter() {
                instance.create_cf(cf, &options).unwrap();
            }
        }

        Ok(EutxoIndexers {
            db: Arc::new(instance),
        })
    }
}

// implement BlockBatchIndexer trait
impl Indexers for EutxoIndexers {
    type OutBlock = CiBlock;
    fn get_last_height(&self) -> u32 {
        let db_clone = Arc::clone(&self.db);
        eutxo::eutxo_input_indexer::get_last_height(db_clone)
    }

    fn get_indexers(&self) -> Vec<Arc<Mutex<dyn Consumer<Vec<CiBlock>>>>> {
        vec![Arc::new(Mutex::new(
            eutxo::eutxo_input_indexer::EutxoInputIndexer::new(Arc::clone(&self.db)),
        ))]
    }
}
