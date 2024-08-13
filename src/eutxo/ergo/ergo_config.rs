use serde::Deserialize;

use crate::{
    model::{DbIndexCfIndex, DbIndexUtxoBirthPkWithUtxoPkCf, DbIndexValue},
    settings::Indexes,
};

use super::ergo_processor::{OutputAddress, OutputErgoTreeHash, OutputErgoTreeT8Hash};

#[derive(Debug, Deserialize, Clone)]
pub struct ErgoConfig {
    pub db_indexes: ErgoIndexes,
    pub api_host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ErgoIndexes {
    address: bool,
    ergo_tree_hash: bool,
    ergo_tree_t8_hash: bool,
}

impl Indexes<(OutputAddress, OutputErgoTreeHash, OutputErgoTreeT8Hash)> for ErgoIndexes {
    fn get_indexes(&self) -> Vec<DbIndexUtxoBirthPkWithUtxoPkCf> {
        let mut index_names = Vec::new();

        if self.address {
            index_names.push("ADDRESS".to_string());
        }

        if self.ergo_tree_hash {
            index_names.push("ERGO_TREE_HASH".to_string());
        }

        if self.ergo_tree_t8_hash {
            index_names.push("ERGO_TREE_T8_HASH".to_string());
        }

        index_names
    }
    fn create_indexes(
        &self,
        indexes: (OutputAddress, OutputErgoTreeHash, OutputErgoTreeT8Hash),
    ) -> Vec<(DbIndexCfIndex, DbIndexValue)> {
        let mut db_indexes = Vec::with_capacity(2);
        if self.address {
            db_indexes.push((0, indexes.0));
        }
        if self.ergo_tree_hash {
            db_indexes.push((1, indexes.1));
        }
        if self.ergo_tree_t8_hash {
            db_indexes.push((2, indexes.2));
        }
        db_indexes
    }
}
