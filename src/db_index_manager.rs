use crate::model::{DbAgidByIndexCf, DbIndexAgidWithUtxoPkCf, DbIndexByAgidCf};

pub struct DbIndexManager {
    pub index_agid_with_utxo_pk: Vec<DbIndexAgidWithUtxoPkCf>,
    pub agid_by_index: Vec<DbAgidByIndexCf>,
    pub index_by_agid: Vec<DbIndexByAgidCf>,
}

impl DbIndexManager {
    fn new(db_indexes: Vec<DbIndexAgidWithUtxoPkCf>) -> Self {
        let agid_by_index = db_indexes
            .into_iter()
            .map(|index_name| format!("agid_by_{}", index_name))
            .collect();

        let index_by_agid = db_indexes
            .into_iter()
            .map(|index_name| format!("{}_by_agid", index_name))
            .collect();

        DbIndexManager {
            index_agid_with_utxo_pk: db_indexes,
            agid_by_index,
            index_by_agid,
        }
    }
}
