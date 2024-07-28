use std::cell::RefCell;
use std::cell::RefMut;

use rocksdb::ColumnFamily;
use rocksdb::OptimisticTransactionDB;
use rocksdb::Transaction;
use rocksdb::WriteBatchWithTransaction;

// Define a trait for blockchain-specific fields
pub trait ChainFamilies<'db> {}

pub struct SharedFamilies<'db> {
    pub(crate) meta_cf: &'db ColumnFamily,
    pub(crate) block_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) block_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) tx_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) tx_pk_by_hash_cf: &'db ColumnFamily,
}

pub struct RocksDbBatch<'db, CF>
where
    CF: ChainFamilies<'db>,
{
    pub(crate) db_tx: RefCell<Transaction<'db, OptimisticTransactionDB>>,
    pub(crate) batch: RefCell<WriteBatchWithTransaction<true>>,
    pub(crate) shared: SharedFamilies<'db>,
    pub(crate) custom: CF,
}
