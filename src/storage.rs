use std::sync::{Arc, RwLock};

use crate::{db_index_manager::DbIndexManager, info};
use rocksdb::{OptimisticTransactionDB, Options, SingleThreaded, Transaction};

/// Type alias for a database
pub(crate) type Db = OptimisticTransactionDB;

/// Type alias for a transaction
pub(crate) type Tx<'db> = Transaction<'db, Db>;

pub struct Storage {
    pub db: Arc<RwLock<OptimisticTransactionDB<SingleThreaded>>>,
    pub db_index_manager: Arc<DbIndexManager>,
}

impl Storage {
    pub fn new(
        db_path: &str,
        db_index_manager: Arc<DbIndexManager>,
        cfs: Vec<&'static str>,
    ) -> Self {
        let num_cores = num_cpus::get() as i32;
        info!("Number of CPU cores: {}", num_cores);

        let mut opts = Options::default();
        opts.create_if_missing(true);
        // Increase parallelism: setting the number of background threads
        opts.increase_parallelism(num_cores); // Set this based on your CPU cores
        opts.set_max_background_jobs(std::cmp::max(num_cores / 2, 6));
        opts.set_atomic_flush(true);
        // opts.set_allow_mmap_writes(true); // cannot be used together with use_direct_io_for_flush_and_compaction
        opts.set_allow_mmap_reads(true);

        // Set other options for performance
        opts.set_max_file_opening_threads(std::cmp::max(num_cores, 16));
        opts.set_write_buffer_size(128 * 1024 * 1024); // 64 MB
        opts.set_max_write_buffer_number(8);
        opts.set_target_file_size_base(128 * 1024 * 1024); // 64 MB
        opts.set_max_bytes_for_level_base(512 * 1024 * 1024);
        opts.set_use_direct_io_for_flush_and_compaction(true);

        let existing_cfs =
            OptimisticTransactionDB::<SingleThreaded>::list_cf(&opts, db_path).unwrap_or(vec![]);

        let mut db =
            OptimisticTransactionDB::<SingleThreaded>::open_cf(&opts, db_path, &existing_cfs)
                .unwrap();

        if existing_cfs.is_empty() {
            let options = rocksdb::Options::default();
            for cf in cfs.into_iter() {
                info!("Creating column family: {}", cf);
                db.create_cf(cf, &options).unwrap();
            }
            for index_utxo_birth_pk_with_utxo_pk in
                db_index_manager.index_utxo_birth_pk_with_utxo_pk.iter()
            {
                info!(
                    "Creating column family: {}",
                    index_utxo_birth_pk_with_utxo_pk
                );
                db.create_cf(index_utxo_birth_pk_with_utxo_pk, &options)
                    .unwrap();
            }
            for index_by_utxo_birth_pk in db_index_manager.index_by_utxo_birth_pk.iter() {
                info!("Creating column family: {}", index_by_utxo_birth_pk);
                db.create_cf(index_by_utxo_birth_pk, &options).unwrap();
            }
            for utxo_birth_pk_by_index in db_index_manager.utxo_birth_pk_by_index.iter() {
                info!("Creating column family: {}", utxo_birth_pk_by_index);
                db.create_cf(utxo_birth_pk_by_index, &options).unwrap();
            }
        }
        let db = Arc::new(RwLock::new(db));
        Storage {
            db,
            db_index_manager,
        }
    }
}
