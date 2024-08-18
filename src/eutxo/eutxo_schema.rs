use indexmap::IndexMap;
use serde::Deserialize;

use std::{collections::HashMap, fs};

use crate::model::CompactionEnabled;

pub type DbIndexNumber = u8;
pub type DbIndexValueSize = u16;
pub type DbIndexName = String;
pub type DbIndexEnabled = bool;

#[derive(Debug, Deserialize)]
struct DbOutputIndexInfo {
    enabled: DbIndexEnabled,
}

#[derive(Debug, Deserialize)]
struct SchemaDefinition {
    one_to_many_index: IndexMap<DbIndexName, DbOutputIndexInfo>,
    one_to_one_index: Option<IndexMap<DbIndexName, DbOutputIndexInfo>>,
}

#[derive(Debug, Deserialize)]
struct SchemaDefinitionHolder {
    bitcoin: SchemaDefinition,
    cardano: SchemaDefinition,
    ergo: SchemaDefinition,
}

impl From<SchemaDefinitionHolder> for DbSchemaHolder {
    fn from(raw: SchemaDefinitionHolder) -> Self {
        DbSchemaHolder {
            bitcoin: DbSchema::new(raw.bitcoin),
            cardano: DbSchema::new(raw.cardano),
            ergo: DbSchema::new(raw.ergo),
        }
    }
}

#[derive(Debug, Clone)]
pub struct O2mIndexNameByNumber {
    pub utxo_birth_pk_relations: Vec<(DbIndexNumber, DbIndexName, CompactionEnabled)>,
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbIndexName, CompactionEnabled)>,
    pub index_by_utxo_birth_pk: Vec<(DbIndexNumber, DbIndexName, CompactionEnabled)>,
}

#[derive(Debug, Clone)]
pub struct O2oIndexNameByNumber {
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbIndexName, CompactionEnabled)>,
}

#[derive(Debug)]
pub struct DbSchemaHolder {
    pub bitcoin: DbSchema,
    pub cardano: DbSchema,
    pub ergo: DbSchema,
}

#[derive(Debug, Clone)]
pub struct DbSchema {
    pub o2m_index_number_by_name: HashMap<DbIndexName, DbIndexNumber>,
    pub o2o_index_number_by_name: HashMap<DbIndexName, DbIndexNumber>,
    pub o2m_index_name_by_number: O2mIndexNameByNumber,
    pub o2o_index_name_by_number: O2oIndexNameByNumber,
}

impl DbSchema {
    pub fn load_config(path: &str) -> DbSchemaHolder {
        let yaml_str = fs::read_to_string(path).expect("Failed to read YAML file");
        let raw_config: SchemaDefinitionHolder =
            serde_yaml::from_str(&yaml_str).expect("Failed to parse YAML");
        raw_config.into()
    }

    fn new(raw: SchemaDefinition) -> Self {
        let o2m_index_number_by_name: HashMap<DbIndexName, DbIndexNumber> = raw
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

        let o2o_index_number_by_name = raw
            .one_to_one_index
            .map(|index_map| {
                index_map
                    .into_iter()
                    .zip((0..=255).rev())
                    .filter_map(|((db_index_name, db_index_info), index_number)| {
                        if db_index_info.enabled {
                            Some((db_index_name, index_number as u8))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<DbIndexName, DbIndexNumber>>()
            })
            .unwrap_or_default();

        DbSchema {
            o2m_index_number_by_name: o2m_index_number_by_name.clone(),
            o2o_index_number_by_name: o2o_index_number_by_name.clone(),
            o2m_index_name_by_number: O2mIndexNameByNumber {
                utxo_birth_pk_relations: o2m_index_number_by_name
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2M_{}_RELATIONS", *index_name),
                            false,
                        )
                    })
                    .collect(),
                utxo_birth_pk_by_index: o2m_index_number_by_name
                    .iter()
                    .map(|(index_name, index_number)| {
                        (
                            *index_number,
                            format!("O2M_UTXO_BIRTH_PK_BY_{}", *index_name),
                            true,
                        )
                    })
                    .collect(),
                index_by_utxo_birth_pk: o2m_index_number_by_name
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
            o2o_index_name_by_number: O2oIndexNameByNumber {
                utxo_birth_pk_by_index: o2o_index_number_by_name
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

    #[test]
    fn test_cardano_indexes() {
        let config = DbSchema::load_config("config/schema.yaml");
        assert!(config
            .cardano
            .o2m_index_number_by_name
            .contains_key("ADDRESS"));
        assert!(!config
            .cardano
            .o2m_index_number_by_name
            .contains_key("SCRIPT_HASH"));
        assert!(
            config.cardano.o2o_index_number_by_name.is_empty(),
            "Expected one_to_one_index to be empty"
        );
    }

    #[test]
    fn test_ergo_indexes() {
        let config = DbSchema::load_config("config/schema.yaml");
        assert!(config.ergo.o2m_index_number_by_name.contains_key("ADDRESS"));
        assert!(!config
            .ergo
            .o2m_index_number_by_name
            .contains_key("ERGO_TREE_HASH"));
        assert!(!config
            .ergo
            .o2m_index_number_by_name
            .contains_key("ERGO_TREE_T8_HASH"));
        assert_eq!(config.ergo.o2o_index_number_by_name.get("BOX_ID"), Some(&0));
    }
}
