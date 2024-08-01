use rocksdb::{MultiThreaded, OptimisticTransactionDB, Options};

use crate::{
    db_options,
    eutxo::{eutxo_index_manager::DbIndexManager, eutxo_model},
    info, model,
};

pub fn get_db(
    db_index_manager: &DbIndexManager,
    db_path: &str,
) -> OptimisticTransactionDB<MultiThreaded> {
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
        for (cf, compaction) in all_cfs.into_iter() {
            info!("Creating column family {}, compaction {}", cf, compaction);
            if compaction {
                db.create_cf(cf, &cf_compaction_enabled_opts).unwrap();
            } else {
                db.create_cf(cf, &options).unwrap();
            }
        }
        for index_utxo_birth_pk_with_utxo_pk in db_index_manager.utxo_birth_pk_relations.iter() {
            info!(
                "Creating column family {}, compaction {}",
                index_utxo_birth_pk_with_utxo_pk, false
            );
            db.create_cf(index_utxo_birth_pk_with_utxo_pk, &options)
                .unwrap();
        }
        for index_by_utxo_birth_pk in db_index_manager.index_by_utxo_birth_pk.iter() {
            info!(
                "Creating column family {}, compaction {}",
                index_by_utxo_birth_pk, false
            );
            db.create_cf(index_by_utxo_birth_pk, &options).unwrap();
        }
        for utxo_birth_pk_by_index in db_index_manager.utxo_birth_pk_by_index.iter() {
            info!(
                "Creating column family {}, compaction {}",
                utxo_birth_pk_by_index, true
            );
            db.create_cf(utxo_birth_pk_by_index, &cf_compaction_enabled_opts)
                .unwrap();
        }
    }
    db
}
