use rocksdb::{ColumnFamilyDescriptor, MultiThreaded, OptimisticTransactionDB};

use crate::db_options;

use super::eutxo_schema::DbSchema;

pub fn get_db(db_schema: &DbSchema, db_path: &str) -> OptimisticTransactionDB<MultiThreaded> {
    let options = db_options::get_db_options(false, None);
    let existing_cfs =
        OptimisticTransactionDB::<MultiThreaded>::list_cf(&options, db_path).unwrap_or_default();

    if !existing_cfs.is_empty() {
        let cf_descriptors = db_schema
            .get_cf_names_with_options()
            .into_iter()
            .map(|(cf, options)| ColumnFamilyDescriptor::new(cf, options));

        OptimisticTransactionDB::<MultiThreaded>::open_cf_descriptors(
            &options,
            db_path,
            cf_descriptors,
        )
        .unwrap()
    } else {
        let db =
            OptimisticTransactionDB::<MultiThreaded>::open_cf(&options, db_path, &existing_cfs)
                .unwrap();

        db_schema
            .get_cf_names_with_options()
            .iter()
            .for_each(|(cf, options)| db.create_cf(cf, options).unwrap());
        db
    }
}
