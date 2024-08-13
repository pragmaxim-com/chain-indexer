use serde::Deserialize;

use crate::{
    eutxo::btc::btc_processor::{OutputAddress, OutputScriptHash},
    model::{DbIndexCfIndex, DbIndexUtxoBirthPkWithUtxoPkCf, DbIndexValue},
    settings::Indexes,
};

#[derive(Debug, Deserialize, Clone)]
pub struct BitcoinConfig {
    pub db_indexes: BitcoinIndexes,
    pub api_host: String,
    pub api_username: String,
    pub api_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BitcoinIndexes {
    script_hash: bool,
    address: bool,
}

impl Indexes<(OutputAddress, OutputScriptHash)> for BitcoinIndexes {
    fn get_indexes(&self) -> Vec<DbIndexUtxoBirthPkWithUtxoPkCf> {
        let mut index_names = Vec::new();

        if self.script_hash {
            index_names.push("SCRIPT_HASH".to_string());
        }

        if self.address {
            index_names.push("ADDRESS".to_string());
        }

        index_names
    }

    fn create_indexes(
        &self,
        indexes: (OutputAddress, OutputScriptHash),
    ) -> Vec<(DbIndexCfIndex, DbIndexValue)> {
        let mut db_indexes = Vec::with_capacity(2);
        if self.address && indexes.0.is_some() {
            db_indexes.push((0, indexes.0.unwrap()));
        }
        if self.script_hash {
            db_indexes.push((1, indexes.1));
        }
        db_indexes
    }
}
