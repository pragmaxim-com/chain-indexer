use std::{collections::HashMap, sync::Arc};

use model::eutxo_model::DbIndexNumber;
use rocksdb::BoundColumnFamily;

use crate::rocks_db_batch::CustomFamilies;

#[derive(Clone)]
pub struct EutxoFamilies {
    pub utxo_value_by_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub utxo_pk_by_input_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub input_pk_by_utxo_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub o2m_utxo_birth_pk_relations_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'static>>>,
    pub o2m_utxo_birth_pk_by_index_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'static>>>,
    pub o2m_index_by_utxo_birth_pk_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'static>>>,
    pub o2o_utxo_birth_pk_by_index_cf: HashMap<DbIndexNumber, Arc<BoundColumnFamily<'static>>>,
    pub assets_by_utxo_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub asset_id_by_asset_birth_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub asset_birth_pk_by_asset_id_cf: Arc<BoundColumnFamily<'static>>,
    pub asset_birth_pk_relations_cf: Arc<BoundColumnFamily<'static>>,
}

impl CustomFamilies for EutxoFamilies {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'static>>> {
        let mut all = vec![
            Arc::clone(&self.utxo_value_by_pk_cf),
            Arc::clone(&self.utxo_pk_by_input_pk_cf),
            Arc::clone(&self.input_pk_by_utxo_pk_cf),
            Arc::clone(&self.assets_by_utxo_pk_cf),
            Arc::clone(&self.asset_id_by_asset_birth_pk_cf),
            Arc::clone(&self.asset_birth_pk_by_asset_id_cf),
            Arc::clone(&self.asset_birth_pk_relations_cf),
        ];

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
