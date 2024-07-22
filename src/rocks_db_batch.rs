use crate::storage::Tx;
use rocksdb::ColumnFamily;
use rocksdb::WriteBatchWithTransaction;

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
    pub(crate) utxo_birth_pk_with_utxo_pk_cf: Vec<&'db ColumnFamily>, // TODO eutxo !!
    pub(crate) utxo_birth_pk_by_index_cf: Vec<&'db ColumnFamily>,     // TODO eutxo !!
    pub(crate) index_by_utxo_birth_pk_cf: Vec<&'db ColumnFamily>,
    pub(crate) assets_by_utxo_pk_cf: &'db ColumnFamily,
    pub(crate) asset_id_by_asset_birth_pk_cf: &'db ColumnFamily,
    pub(crate) asset_birth_pk_by_asset_id_cf: &'db ColumnFamily,
    pub(crate) asset_birth_pk_with_asset_pk_cf: &'db ColumnFamily,
}
