use std::{collections::HashMap, sync::Arc};

use rocksdb::BoundColumnFamily;

use crate::rocks_db_batch::CustomFamilies;

use super::eutxo_schema::DbIndexNumber;

#[derive(Clone)]
pub struct EutxoFamilies<'db> {
    pub utxo_value_by_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub utxo_pk_by_input_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub input_pk_by_utxo_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub o2m_utxo_birth_pk_relations_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'db>>>,
    pub o2m_utxo_birth_pk_by_index_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'db>>>,
    pub o2m_index_by_utxo_birth_pk_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'db>>>,
    pub o2o_utxo_birth_pk_by_index_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'db>>>,
    pub assets_by_utxo_pk_cf: Arc<BoundColumnFamily<'db>>,
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
        all.push(Arc::clone(&self.assets_by_utxo_pk_cf));
        all.push(Arc::clone(&self.asset_id_by_asset_birth_pk_cf));
        all.push(Arc::clone(&self.asset_birth_pk_by_asset_id_cf));
        all.push(Arc::clone(&self.asset_birth_pk_with_asset_pk_cf));

        for (_, x) in self.o2m_utxo_birth_pk_relations_cf.iter() {
            all.push(Arc::clone(x));
        }

        for (_, x) in self.o2m_utxo_birth_pk_by_index_cf.iter() {
            all.push(Arc::clone(x));
        }

        for (_, x) in self.o2m_index_by_utxo_birth_pk_cf.iter() {
            all.push(Arc::clone(x));
        }
        all
    }
}
