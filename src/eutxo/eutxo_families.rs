use std::sync::Arc;

use rocksdb::BoundColumnFamily;

use crate::rocks_db_batch::CustomFamilies;

#[derive(Clone)]
pub struct EutxoFamilies<'db> {
    pub utxo_value_by_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub utxo_pk_by_input_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub input_pk_by_utxo_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub utxo_birth_pk_with_utxo_pk_cf: Vec<Arc<BoundColumnFamily<'db>>>,
    pub utxo_birth_pk_by_index_cf: Vec<Arc<BoundColumnFamily<'db>>>,
    pub index_by_utxo_birth_pk_cf: Vec<Arc<BoundColumnFamily<'db>>>,
    pub asset_by_asset_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub asset_id_by_asset_birth_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub asset_birth_pk_by_asset_id_cf: Arc<BoundColumnFamily<'db>>,
    pub asset_birth_pk_with_asset_pk_cf: Arc<BoundColumnFamily<'db>>,
}

impl<'db> CustomFamilies<'db> for EutxoFamilies<'db> {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'db>>> {
        let mut all = vec![];
        all.push(Arc::clone(&self.utxo_value_by_pk_cf));
        all.push(Arc::clone(&self.utxo_pk_by_input_pk_cf));
        all.push(Arc::clone(&self.input_pk_by_utxo_pk_cf));
        all.push(Arc::clone(&self.asset_by_asset_pk_cf));
        all.push(Arc::clone(&self.asset_id_by_asset_birth_pk_cf));
        all.push(Arc::clone(&self.asset_birth_pk_by_asset_id_cf));
        all.push(Arc::clone(&self.asset_birth_pk_with_asset_pk_cf));

        for x in self.utxo_birth_pk_with_utxo_pk_cf.iter() {
            all.push(Arc::clone(x));
        }

        for x in self.utxo_birth_pk_by_index_cf.iter() {
            all.push(Arc::clone(x));
        }

        for x in self.index_by_utxo_birth_pk_cf.iter() {
            all.push(Arc::clone(x));
        }
        all
    }
}
