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
            .map(|index_name| format!("UTXO_BIRTH_PK_BY_{}", index_name))
            .collect();

        let index_by_utxo_birth_pk = db_indexes
            .into_iter()
            .map(|index_name| format!("{}_BY_UTXO_BIRTH_PK", index_name))
            .collect();

        let utxo_birth_pk_relations = db_indexes
            .into_iter()
            .map(|index_name| format!("{}_RELATIONS", index_name))
            .collect();

        DbIndexManager {
            utxo_birth_pk_relations,
            utxo_birth_pk_by_index,
            index_by_utxo_birth_pk,
        }
    }
}
