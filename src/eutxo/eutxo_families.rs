use rocksdb::ColumnFamily;

use crate::rocks_db_batch::CustomFamilies;

pub struct EutxoFamilies<'db> {
    pub(crate) utxo_value_by_pk_cf: &'db ColumnFamily,
    pub(crate) utxo_pk_by_input_pk_cf: &'db ColumnFamily,
    pub(crate) utxo_birth_pk_with_utxo_pk_cf: Vec<&'db ColumnFamily>,
    pub(crate) utxo_birth_pk_by_index_cf: Vec<&'db ColumnFamily>,
    pub(crate) index_by_utxo_birth_pk_cf: Vec<&'db ColumnFamily>,
    pub(crate) assets_by_utxo_pk_cf: &'db ColumnFamily,
    pub(crate) asset_id_by_asset_birth_pk_cf: &'db ColumnFamily,
    pub(crate) asset_birth_pk_by_asset_id_cf: &'db ColumnFamily,
    pub(crate) asset_birth_pk_with_asset_pk_cf: &'db ColumnFamily,
}

impl<'db> CustomFamilies<'db> for EutxoFamilies<'db> {}
