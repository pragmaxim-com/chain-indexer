use rocksdb::{MultiThreaded, OptimisticTransactionDB, Options};

use crate::{db_options, eutxo::eutxo_model, info, model};

use super::eutxo_schema::DbSchema;

pub fn get_db(db_schema: &DbSchema, db_path: &str) -> OptimisticTransactionDB<MultiThreaded> {
    let options = db_options::get_db_options();
    let existing_cfs =
        OptimisticTransactionDB::<MultiThreaded>::list_cf(&options, &db_path).unwrap_or(vec![]);

    let db = OptimisticTransactionDB::<MultiThreaded>::open_cf(&options, &db_path, &existing_cfs)
        .unwrap();

    if existing_cfs.is_empty() {
        let mut cf_compaction_enabled_opts = Options::default();
        cf_compaction_enabled_opts.set_disable_auto_compactions(false);
        let shared_cfs = model::get_shared_column_families();
        let eutxo_cfs = eutxo_model::get_eutxo_column_families();
        let all_cfs = [shared_cfs, eutxo_cfs].concat();

        let get_opts = |compaction_enabled: bool| -> &Options {
            if compaction_enabled {
                &options
            } else {
                &cf_compaction_enabled_opts
            }
        };

        for (cf, compation_enabled) in all_cfs.into_iter() {
            info!(
                "Creating column family {}, compaction {}",
                cf, compation_enabled
            );
            db.create_cf(cf, get_opts(compation_enabled)).unwrap();
        }

        // One To Many
        for (_, index_utxo_birth_pk_with_utxo_pk, compaction_enabled) in db_schema
            .one_to_many_index_cfs
            .utxo_birth_pk_relations
            .iter()
        {
            info!(
                "Creating one-to-many index column family {}, compaction {}",
                index_utxo_birth_pk_with_utxo_pk, compaction_enabled
            );
            db.create_cf(
                index_utxo_birth_pk_with_utxo_pk,
                get_opts(*compaction_enabled),
            )
            .unwrap();
        }
        for (_, index_by_utxo_birth_pk, compaction_enabled) in db_schema
            .one_to_many_index_cfs
            .index_by_utxo_birth_pk
            .iter()
        {
            info!(
                "Creating one-to-many index column family {}, compaction {}",
                index_by_utxo_birth_pk, compaction_enabled
            );
            db.create_cf(index_by_utxo_birth_pk, get_opts(*compaction_enabled))
                .unwrap();
        }
        for (_, utxo_birth_pk_by_index, compaction_enabled) in db_schema
            .one_to_many_index_cfs
            .utxo_birth_pk_by_index
            .iter()
        {
            info!(
                "Creating one-to-many index column family {}, compaction {}",
                utxo_birth_pk_by_index, compaction_enabled
            );
            db.create_cf(utxo_birth_pk_by_index, get_opts(*compaction_enabled))
                .unwrap();
        }

        // One To One
        for (_, utxo_birth_pk_by_index, compaction_enabled) in
            db_schema.one_to_one_index_cfs.utxo_birth_pk_by_index.iter()
        {
            info!(
                "Creating one-to-one index column family {}, compaction {}",
                utxo_birth_pk_by_index, compaction_enabled
            );
            db.create_cf(utxo_birth_pk_by_index, get_opts(*compaction_enabled))
                .unwrap();
        }
    }
    db
}
