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

impl<'db> CustomFamilies<'db> for EutxoFamilies<'db> {
    fn get_all(&self) -> Vec<&'db ColumnFamily> {
        let mut all = vec![];
        all.push(self.utxo_value_by_pk_cf);
        all.push(self.utxo_pk_by_input_pk_cf);
        all.push(self.assets_by_utxo_pk_cf);
        all.push(self.asset_id_by_asset_birth_pk_cf);
        all.push(self.asset_birth_pk_by_asset_id_cf);
        all.push(self.asset_birth_pk_with_asset_pk_cf);

        for x in self.utxo_birth_pk_with_utxo_pk_cf.iter() {
            all.push(*x);
        }

        for x in self.utxo_birth_pk_by_index_cf.iter() {
            all.push(*x);
        }

        for x in self.index_by_utxo_birth_pk_cf.iter() {
            all.push(*x);
        }
        all
    }
}
