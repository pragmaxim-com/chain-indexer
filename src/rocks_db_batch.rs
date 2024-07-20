use rocksdb::ColumnFamily;
use rocksdb::WriteBatchWithTransaction;
use std::sync::Arc;

use crate::eutxo::eutxo_model::*;
use crate::storage::Storage;
use crate::storage::Tx;

pub struct RocksDbBatch<'db> {
    pub(crate) db_tx: &'db Tx<'db>,
    pub(crate) batch: &'db mut WriteBatchWithTransaction<true>,
    pub(crate) block_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) block_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) tx_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) tx_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) utxo_value_by_pk_cf: &'db ColumnFamily, // TODO eutxo !!
    pub(crate) utxo_pk_by_input_pk_cf: &'db ColumnFamily, // TODO eutxo !!
    pub(crate) meta_cf: &'db ColumnFamily,
    pub(crate) utxo_birth_pk_with_utxo_pk_cf: Vec<&'db ColumnFamily>,
    pub(crate) index_by_utxo_birth_pk_cf: Vec<&'db ColumnFamily>,
    pub(crate) utxo_birth_pk_by_index_cf: Vec<&'db ColumnFamily>,
}

impl<'db> RocksDbBatch<'db> {
    pub(crate) fn new(db_holder: Arc<Storage>) -> Self {
        let db = db_holder.db.write().unwrap();
        let db_tx = db.transaction();
        let mut binding = db_tx.get_writebatch();
        RocksDbBatch {
            db_tx: &db_tx,
            batch: &mut binding,
            block_hash_by_pk_cf: db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
            block_pk_by_hash_cf: db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
            tx_hash_by_pk_cf: db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
            tx_pk_by_hash_cf: db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
            utxo_value_by_pk_cf: db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
            utxo_pk_by_input_pk_cf: db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
            meta_cf: db.cf_handle(META_CF).unwrap(),
            utxo_birth_pk_with_utxo_pk_cf: db_holder
                .db_index_manager
                .index_utxo_birth_pk_with_utxo_pk
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            utxo_birth_pk_by_index_cf: db_holder
                .db_index_manager
                .utxo_birth_pk_by_index
                .iter()
                .map(|cf| db.cf_handle(cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
            index_by_utxo_birth_pk_cf: db_holder
                .db_index_manager
                .index_by_utxo_birth_pk
                .iter()
                .map(|cf| db.cf_handle(&cf).unwrap())
                .collect::<Vec<&ColumnFamily>>(),
        }
    }

    pub(crate) fn commit(&self) -> Result<(), rocksdb::Error> {
        self.db_tx.commit()
    }
}
