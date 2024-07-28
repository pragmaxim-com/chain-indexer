use crate::model::{
    DbIndexByUtxoBirthPkCf, DbIndexUtxoBirthPkWithUtxoPkCf, DbUtxoBirthPkByIndexCf,
};

pub struct DbIndexManager {
    pub utxo_birth_pk_relations: Vec<DbIndexUtxoBirthPkWithUtxoPkCf>,
    pub utxo_birth_pk_by_index: Vec<DbUtxoBirthPkByIndexCf>,
    pub index_by_utxo_birth_pk: Vec<DbIndexByUtxoBirthPkCf>,
}

impl DbIndexManager {
    pub fn new(db_indexes: &Vec<DbIndexUtxoBirthPkWithUtxoPkCf>) -> Self {
        let utxo_birth_pk_by_index = db_indexes
            .into_iter()
            .map(|index_name| format!("utxo_birth_pk_by_{}", index_name))
            .collect();

        let index_by_utxo_birth_pk = db_indexes
            .into_iter()
            .map(|index_name| format!("{}_by_utxo_birth_pk", index_name))
            .collect();

        DbIndexManager {
            utxo_birth_pk_relations: db_indexes.clone(),
            utxo_birth_pk_by_index,
            index_by_utxo_birth_pk,
        }
    }
}
