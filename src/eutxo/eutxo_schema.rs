use serde::Deserialize;

use std::collections::HashMap;

use crate::model::{
    CompactionEnabled, DbIndexByUtxoBirthPkCf, DbIndexUtxoBirthPkWithUtxoPkCf,
    DbUtxoBirthPkByIndexCf,
};

pub type DbIndexNumber = u8;
pub type DbIndexName = String;
pub type DbIndexEnabled = bool;

#[derive(Debug, Deserialize)]
struct DbOutputIndexInfo {
    enabled: DbIndexEnabled,
}

#[derive(Debug, Deserialize)]
struct RawOutputIndexes {
    one_to_many_index: Vec<(DbIndexName, DbOutputIndexInfo)>,
    one_to_one_index: Vec<(DbIndexName, DbOutputIndexInfo)>,
}

#[derive(Debug, Deserialize)]
struct RawSchema {
    bitcoin: RawOutputIndexes,
    cardano: RawOutputIndexes,
    ergo: RawOutputIndexes,
}

impl From<RawOutputIndexes> for DbOutputIndexLayout {
    fn from(raw: RawOutputIndexes) -> Self {
        let one_to_many_index = raw
            .one_to_many_index
            .into_iter()
            .enumerate()
            .filter_map(|(index_number, (db_index_name, db_index_info))| {
                if db_index_info.enabled {
                    Some((db_index_name, index_number as u8))
                } else {
                    None
                }
            })
            .collect();

        let one_to_one_index = raw
            .one_to_one_index
            .into_iter()
            .zip((0..=255).rev())
            .filter_map(|((db_index_name, db_index_info), index_number)| {
                if db_index_info.enabled {
                    Some((db_index_name, index_number as u8))
                } else {
                    None
                }
            })
            .collect();

        DbOutputIndexLayout {
            one_to_many: one_to_many_index,
            one_to_one: one_to_one_index,
        }
    }
}

impl From<RawSchema> for DbSchemaHolder {
    fn from(raw: RawSchema) -> Self {
        DbSchemaHolder {
            bitcoin: DbSchema::new(raw.bitcoin.into()),
            cardano: DbSchema::new(raw.cardano.into()),
            ergo: DbSchema::new(raw.ergo.into()),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbOutputIndexLayout {
    pub one_to_many: HashMap<DbIndexName, DbIndexNumber>,
    pub one_to_one: HashMap<DbIndexName, DbIndexNumber>,
}

#[derive(Debug, Clone)]
pub struct O2mOutputIndexCfs {
    pub utxo_birth_pk_relations: Vec<(
        DbIndexNumber,
        DbIndexUtxoBirthPkWithUtxoPkCf,
        CompactionEnabled,
    )>,
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbUtxoBirthPkByIndexCf, CompactionEnabled)>,
    pub index_by_utxo_birth_pk: Vec<(DbIndexNumber, DbIndexByUtxoBirthPkCf, CompactionEnabled)>,
}

#[derive(Debug, Clone)]
pub struct O2oOutputIndexCfs {
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbUtxoBirthPkByIndexCf, CompactionEnabled)>,
}

#[derive(Debug)]
pub struct DbSchemaHolder {
    pub bitcoin: DbSchema,
    pub cardano: DbSchema,
    pub ergo: DbSchema,
}

#[derive(Debug, Clone)]
pub struct DbSchema {
    pub db_index_table: DbOutputIndexLayout,
    pub one_to_many_index_cfs: O2mOutputIndexCfs,
    pub one_to_one_index_cfs: O2oOutputIndexCfs,
}

impl DbSchema {
    pub fn new(db_index_table: DbOutputIndexLayout) -> Self {
        DbSchema {
            db_index_table: db_index_table.clone(),
            one_to_many_index_cfs: O2mOutputIndexCfs {
                utxo_birth_pk_relations: db_index_table
                    .one_to_many
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2M_{}_RELATIONS", *index_name),
                            false,
                        )
                    })
                    .collect(),
                utxo_birth_pk_by_index: db_index_table
                    .one_to_many
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2M_UTXO_BIRTH_PK_BY_{}", *index_name),
                            true,
                        )
                    })
                    .collect(),
                index_by_utxo_birth_pk: db_index_table
                    .one_to_many
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2M_{}_BY_UTXO_BIRTH_PK", *index_name),
                            false,
                        )
                    })
                    .collect(),
            },
            one_to_one_index_cfs: O2oOutputIndexCfs {
                utxo_birth_pk_by_index: db_index_table
                    .one_to_one
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2O_UTXO_BIRTH_PK_BY_{}", *index_name),
                            true,
                        )
                    })
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn load_config(path: &str) -> DbSchemaHolder {
        let yaml_str = fs::read_to_string(path).expect("Failed to read YAML file");
        let raw_config: RawSchema = serde_yaml::from_str(&yaml_str).expect("Failed to parse YAML");
        raw_config.into()
    }

    #[test]
    fn test_cardano_indexes() {
        let config = load_config("config/schema.yaml");
        assert!(config
            .cardano
            .db_index_table
            .one_to_many
            .contains_key("ADDRESS"));
        assert!(!config
            .cardano
            .db_index_table
            .one_to_many
            .contains_key("SCRIPT_HASH"));
        assert!(
            config.cardano.db_index_table.one_to_one.is_empty(),
            "Expected one_to_one_index to be empty"
        );
    }

    #[test]
    fn test_ergo_indexes() {
        let config = load_config("config/schema.yaml");
        assert!(config
            .ergo
            .db_index_table
            .one_to_many
            .contains_key("ADDRESS"));
        assert!(!config
            .ergo
            .db_index_table
            .one_to_many
            .contains_key("ERGO_TREE_HASH"));
        assert!(!config
            .ergo
            .db_index_table
            .one_to_many
            .contains_key("ERGO_TREE_T8_HASH"));
        assert_eq!(
            config.ergo.db_index_table.one_to_one.get("BOX_ID"),
            Some(&0)
        );
    }
}
