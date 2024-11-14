use indexmap::IndexMap;
use model::eutxo_model::{DbIndexEnabled, DbIndexName, DbIndexNumber};
use rocksdb::{Options, SliceTransform};
use serde::Deserialize;

use std::{collections::HashMap, fs, mem::size_of};

use crate::db_options;

use super::eutxo_codec_utxo::UtxoBirthPkBytes;

pub type CompactionEnabled = bool;

pub const META_CF: &str = "META_CF";
pub const BLOCK_HASH_BY_PK_CF: &str = "BLOCK_HASH_BY_PK_CF";
pub const BLOCK_PK_BY_HASH_CF: &str = "BLOCK_PK_BY_HASH_CF";
pub const TX_HASH_BY_PK_CF: &str = "TX_HASH_BY_PK_CF";
pub const TX_PK_BY_HASH_CF: &str = "TX_PK_BY_HASH_CF";

pub const UTXO_VALUE_BY_PK_CF: &str = "UTXO_VALUE_BY_PK_CF";
pub const UTXO_PK_BY_INPUT_PK_CF: &str = "UTXO_PK_BY_INPUT_PK_CF";
pub const INPUT_PK_BY_UTXO_PK_CF: &str = "INPUT_PK_BY_UTXO_PK_CF";
pub const ASSET_BY_ASSET_PK_CF: &str = "ASSET_BY_ASSET_PK_CF";
pub const ASSET_ID_BY_ASSET_BIRTH_PK_CF: &str = "ASSET_ID_BY_ASSET_BIRTH_PK_CF";
pub const ASSET_BIRTH_PK_BY_ASSET_ID_CF: &str = "ASSET_BIRTH_PK_BY_ASSET_ID_CF";
pub const ASSET_BIRTH_PK_RELATIONS_CF: &str = "ASSET_BIRTH_PK_RELATIONS_CF";

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

#[derive(Clone)]
pub struct O2mIndexNameByNumber {
    pub utxo_birth_pk_relations: Vec<(DbIndexNumber, DbIndexName, Options)>,
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbIndexName, Options)>,
    pub index_by_utxo_birth_pk: Vec<(DbIndexNumber, DbIndexName, Options)>,
}

#[derive(Clone)]
pub struct O2oIndexNameByNumber {
    pub utxo_birth_pk_by_index: Vec<(DbIndexNumber, DbIndexName, Options)>,
}

pub struct DbSchemaHolder {
    pub bitcoin: DbSchema,
    pub cardano: DbSchema,
    pub ergo: DbSchema,
}

#[derive(Clone)]
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
                        let extractor =
                            SliceTransform::create_fixed_prefix(size_of::<UtxoBirthPkBytes>());
                        let options: Options = db_options::get_db_options(false, Some(extractor));
                        (
                            *index_number,
                            format!("O2M_{}_RELATIONS", *index_name),
                            options,
                        )
                    })
                    .collect(),
                utxo_birth_pk_by_index: o2m_index_number_by_name
                    .iter()
                    .map(|(index_name, index_number)| {
                        let options: Options = db_options::get_db_options(false, None);
                        (
                            *index_number,
                            format!("O2M_UTXO_BIRTH_PK_BY_{}", *index_name),
                            options,
                        )
                    })
                    .collect(),
                index_by_utxo_birth_pk: o2m_index_number_by_name
                    .iter()
                    .map(|(index_name, index_number)| {
                        let options: Options = db_options::get_db_options(false, None);
                        (
                            *index_number,
                            format!("O2M_{}_BY_UTXO_BIRTH_PK", *index_name),
                            options,
                        )
                    })
                    .collect(),
            },
            o2o_index_name_by_number: O2oIndexNameByNumber {
                utxo_birth_pk_by_index: o2o_index_number_by_name
                    .iter()
                    .map(|(index_name, index_number)| {
                        let options: Options = db_options::get_db_options(false, None);
                        (
                            *index_number,
                            format!("O2O_UTXO_BIRTH_PK_BY_{}", *index_name),
                            options,
                        )
                    })
                    .collect(),
            },
        }
    }

    pub fn get_cf_names_with_options(&self) -> Vec<(&str, Options)> {
        let mut result = Vec::new();
        let static_cfs = vec![
            (META_CF, false, None),
            (BLOCK_HASH_BY_PK_CF, false, None),
            (BLOCK_PK_BY_HASH_CF, false, None),
            (
                TX_HASH_BY_PK_CF,
                false,
                Some(SliceTransform::create_fixed_prefix(4)),
            ),
            (TX_PK_BY_HASH_CF, false, None),
            (
                UTXO_VALUE_BY_PK_CF,
                false,
                Some(SliceTransform::create_fixed_prefix(6)),
            ),
            (
                UTXO_PK_BY_INPUT_PK_CF,
                false,
                Some(SliceTransform::create_fixed_prefix(6)),
            ),
            (INPUT_PK_BY_UTXO_PK_CF, false, None),
            (ASSET_BY_ASSET_PK_CF, false, None),
            (ASSET_ID_BY_ASSET_BIRTH_PK_CF, false, None),
            (ASSET_BIRTH_PK_BY_ASSET_ID_CF, false, None),
            (ASSET_BIRTH_PK_RELATIONS_CF, false, None),
        ];
        result.extend(
            static_cfs
                .into_iter()
                .map(|(cf, disable_autocompaction, extractor)| {
                    (
                        cf,
                        db_options::get_db_options(disable_autocompaction, extractor),
                    )
                }),
        );
        // One To Many
        for (_, index_utxo_birth_pk_with_utxo_pk, options) in
            self.o2m_index_name_by_number.utxo_birth_pk_relations.iter()
        {
            result.push((index_utxo_birth_pk_with_utxo_pk.as_str(), options.clone()));
        }
        for (_, index_by_utxo_birth_pk, options) in
            self.o2m_index_name_by_number.index_by_utxo_birth_pk.iter()
        {
            result.push((index_by_utxo_birth_pk.as_str(), options.clone()));
        }
        for (_, utxo_birth_pk_by_index, options) in
            self.o2m_index_name_by_number.utxo_birth_pk_by_index.iter()
        {
            result.push((utxo_birth_pk_by_index.as_str(), options.clone()));
        }

        // One To One
        for (_, utxo_birth_pk_by_index, options) in
            self.o2o_index_name_by_number.utxo_birth_pk_by_index.iter()
        {
            result.push((utxo_birth_pk_by_index.as_str(), options.clone()));
        }
        result
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
