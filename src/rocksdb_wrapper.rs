use ouroboros::self_referencing;
use rocksdb::{ColumnFamily, OptimisticTransactionDB, Options, SingleThreaded};

use crate::{api::DbIndexName, info};
use std::sync::Arc;

pub const BLOCK_HASH_BY_PK_CF: &str = "BLOCK_HASH_BY_PK_CF";
pub const BLOCK_PK_BY_HASH_CF: &str = "BLOCK_PK_BY_HASH_CF";
pub const TX_HASH_BY_PK_CF: &str = "TX_HASH_BY_PK_CF";
pub const TX_PK_BY_HASH_CF: &str = "TX_PK_BY_HASH_CF";
pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const META_CF: &str = "META_CF";

#[self_referencing]
pub struct RocksDbWrapper {
    pub(crate) db: OptimisticTransactionDB<SingleThreaded>,
    #[borrows(db)]
    pub(crate) block_hash_by_pk_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) block_pk_by_hash_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) tx_hash_by_pk_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) tx_pk_by_hash_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) utxo_value_by_pk_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) utxo_pk_by_input_pk_cf: &'this ColumnFamily,
    #[borrows(db)]
    pub(crate) meta_cf: &'this ColumnFamily,
    #[borrows(db)]
    #[covariant]
    pub(crate) index_cf_by_name: Vec<(DbIndexName, &'this ColumnFamily)>,
}

impl RocksDbWrapper {
    pub fn initiate(db_path: &str, utxo_indexes: Vec<DbIndexName>) -> Self {
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

        let mut instance =
            OptimisticTransactionDB::<SingleThreaded>::open_cf(&opts, db_path, &existing_cfs)
                .unwrap();

        if existing_cfs.is_empty() {
            let options = rocksdb::Options::default();
            for cf in get_eutxo_column_families().into_iter() {
                info!("Creating column family: {}", cf);
                instance.create_cf(cf, &options).unwrap();
            }
            for cf in utxo_indexes.iter() {
                info!("Creating column family: {}", cf);
                instance.create_cf(cf, &options).unwrap();
            }
        }

        RocksDbWrapperBuilder {
            db: instance,
            block_hash_by_pk_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap()
            },
            block_pk_by_hash_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap()
            },
            tx_hash_by_pk_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(TX_HASH_BY_PK_CF).unwrap()
            },
            tx_pk_by_hash_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(TX_PK_BY_HASH_CF).unwrap()
            },
            utxo_value_by_pk_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap()
            },
            utxo_pk_by_input_pk_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap()
            },
            meta_cf_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                db.cf_handle(META_CF).unwrap()
            },
            index_cf_by_name_builder: |db: &OptimisticTransactionDB<SingleThreaded>| {
                utxo_indexes
                    .iter()
                    .map(|index_name| (index_name.clone(), db.cf_handle(&index_name).unwrap()))
                    .collect()
            },
        }
        .build()
    }
}

pub fn get_eutxo_column_families() -> Vec<&'static str> {
    vec![
        META_CF,
        BLOCK_HASH_BY_PK_CF,
        BLOCK_PK_BY_HASH_CF,
        TX_HASH_BY_PK_CF,
        TX_PK_BY_HASH_CF,
        UTXO_VALUE_BY_PK_CF,
        UTXO_PK_BY_INPUT_PK_CF,
    ]
}
