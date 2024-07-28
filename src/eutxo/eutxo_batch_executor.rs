use std::cell::RefCell;

use rocksdb::{
    ColumnFamily, OptimisticTransactionDB, OptimisticTransactionOptions, Options, SingleThreaded,
    WriteOptions,
};

use crate::{
    api::BatchOperation,
    eutxo::{eutxo_index_manager::DbIndexManager, eutxo_model},
    info,
    model::{self, *},
    rocks_db_batch::{RocksDbBatch, SharedFamilies},
};

use super::{eutxo_families::EutxoFamilies, eutxo_model::*};
pub struct EutxoBatchExecutor<'db> {
    pub db: &'db OptimisticTransactionDB<SingleThreaded>,
    db_index_manager: DbIndexManager,
}

impl<'db> EutxoBatchExecutor<'db> {
    pub fn new(
        db: &'db mut OptimisticTransactionDB<SingleThreaded>,
        options: Options,
        db_indexes: Vec<String>,
        existing_cfs: Vec<String>,
    ) -> Self {
        let db_index_manager = DbIndexManager::new(&db_indexes);
        if existing_cfs.is_empty() {
            let mut cfs: Vec<&str> = model::get_shared_column_families();
            let mut eutxo_cfs: Vec<&str> = eutxo_model::get_eutxo_column_families();
            cfs.append(&mut eutxo_cfs);
            for cf in cfs.into_iter() {
                info!("Creating column family: {}", cf);
                db.create_cf(cf, &options).unwrap();
            }
            for index_utxo_birth_pk_with_utxo_pk in db_index_manager.utxo_birth_pk_relations.iter()
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
        EutxoBatchExecutor {
            db,
            db_index_manager,
        }
    }
}

impl<'db> BatchOperation<'db, EutxoFamilies<'db>> for EutxoBatchExecutor<'db> {
    fn execute(
        &self,
        f: Box<dyn FnOnce(RocksDbBatch<'db, EutxoFamilies<'db>>) -> Result<(), String> + 'db>,
    ) -> Result<(), String> {
        let mut write_options = WriteOptions::default();
        write_options.disable_wal(true);
        let db_tx = self
            .db
            .transaction_opt(&write_options, &OptimisticTransactionOptions::default());
        let batch = db_tx.get_writebatch();

        let batch = RocksDbBatch {
            db_tx: RefCell::new(db_tx),
            batch: RefCell::new(batch),
            shared: SharedFamilies {
                meta_cf: self.db.cf_handle(META_CF).unwrap(),
                block_hash_by_pk_cf: self.db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
                block_pk_by_hash_cf: self.db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
                tx_hash_by_pk_cf: self.db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
                tx_pk_by_hash_cf: self.db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
            },
            custom: EutxoFamilies {
                utxo_value_by_pk_cf: self.db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
                utxo_pk_by_input_pk_cf: self.db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
                utxo_birth_pk_with_utxo_pk_cf: self
                    .db_index_manager
                    .utxo_birth_pk_relations
                    .iter()
                    .map(|cf| self.db.cf_handle(cf).unwrap())
                    .collect::<Vec<&ColumnFamily>>(),
                utxo_birth_pk_by_index_cf: self
                    .db_index_manager
                    .utxo_birth_pk_by_index
                    .iter()
                    .map(|cf| self.db.cf_handle(cf).unwrap())
                    .collect::<Vec<&ColumnFamily>>(),
                index_by_utxo_birth_pk_cf: self
                    .db_index_manager
                    .index_by_utxo_birth_pk
                    .iter()
                    .map(|cf| self.db.cf_handle(&cf).unwrap())
                    .collect::<Vec<&ColumnFamily>>(),
                assets_by_utxo_pk_cf: self.db.cf_handle(ASSETS_BY_UTXO_PK_CF).unwrap(),
                asset_id_by_asset_birth_pk_cf: self
                    .db
                    .cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF)
                    .unwrap(),
                asset_birth_pk_by_asset_id_cf: self
                    .db
                    .cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF)
                    .unwrap(),
                asset_birth_pk_with_asset_pk_cf: self
                    .db
                    .cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF)
                    .unwrap(),
            },
        };

        f(batch);
        Ok(())
    }
}
