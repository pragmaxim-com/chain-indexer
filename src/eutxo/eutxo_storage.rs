use rocksdb::{ColumnFamily, OptimisticTransactionDB, SingleThreaded};

use crate::{
    db_options,
    eutxo::{eutxo_index_manager::DbIndexManager, eutxo_model},
    info,
    model::{self, *},
    rocks_db_batch::{Families, SharedFamilies},
};

use super::{eutxo_families::EutxoFamilies, eutxo_model::*};

pub fn get_db(
    db_index_manager: &DbIndexManager,
    db_path: &str,
) -> OptimisticTransactionDB<SingleThreaded> {
    let options = db_options::get_db_options();
    let existing_cfs =
        OptimisticTransactionDB::<SingleThreaded>::list_cf(&options, &db_path).unwrap_or(vec![]);

    let mut db =
        OptimisticTransactionDB::<SingleThreaded>::open_cf(&options, &db_path, &existing_cfs)
            .unwrap();

    if existing_cfs.is_empty() {
        let mut cfs: Vec<&str> = model::get_shared_column_families();
        let mut eutxo_cfs: Vec<&str> = eutxo_model::get_eutxo_column_families();
        cfs.append(&mut eutxo_cfs);
        for cf in cfs.into_iter() {
            info!("Creating column family: {}", cf);
            db.create_cf(cf, &options).unwrap();
        }
        for index_utxo_birth_pk_with_utxo_pk in db_index_manager.utxo_birth_pk_relations.iter() {
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
    db
}

pub fn get_families<'db>(
    db_index_manager: &DbIndexManager,
    db: &'db OptimisticTransactionDB<SingleThreaded>,
) -> Families<'db, EutxoFamilies<'db>> {
    Families {
        shared: SharedFamilies {
            meta_cf: db.cf_handle(META_CF).unwrap(),
            block_hash_by_pk_cf: db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
            block_pk_by_hash_cf: db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
            tx_hash_by_pk_cf: db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
            tx_pk_by_hash_cf: db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
        },
        custom: EutxoFamilies {
            utxo_value_by_pk_cf: db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
            utxo_pk_by_input_pk_cf: db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
            utxo_birth_pk_with_utxo_pk_cf: db_index_manager
                .utxo_birth_pk_relations
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            utxo_birth_pk_by_index_cf: db_index_manager
                .utxo_birth_pk_by_index
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            index_by_utxo_birth_pk_cf: db_index_manager
                .index_by_utxo_birth_pk
                .iter()
                .map(|cf| db.cf_handle(&cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            assets_by_utxo_pk_cf: db.cf_handle(ASSETS_BY_UTXO_PK_CF).unwrap(),
            asset_id_by_asset_birth_pk_cf: db.cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF).unwrap(),
            asset_birth_pk_by_asset_id_cf: db.cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF).unwrap(),
            asset_birth_pk_with_asset_pk_cf: db.cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF).unwrap(),
        },
    }
}
