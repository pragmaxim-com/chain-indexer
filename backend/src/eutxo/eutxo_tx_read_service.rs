use std::sync::Arc;

use super::{
    eutxo_codec_utxo::{self, UtxoBirthPkBytes},
    eutxo_families::EutxoFamilies,
    eutxo_model::{EuTxInput, EuUtxo, TxHashWithIndex},
};
use crate::{
    api::{ServiceError, TxReadService},
    codec_block,
    codec_tx::{self},
    eutxo::eutxo_model::EuTx,
    persistence::Persistence,
};

use model::{
    eutxo_model::DbIndexNumber, AssetAction, AssetId, AssetValue, BlockHeight, O2mIndexValue,
    TxHash, TxIndex,
};
pub struct EuTxReadService {
    pub(crate) storage: Arc<Persistence<EutxoFamilies>>,
}

impl EuTxReadService {
    pub fn new(storage: Arc<Persistence<EutxoFamilies>>) -> Self {
        EuTxReadService { storage }
    }

    fn get_assets(
        &self,
        birth_pk_bytes: &[u8],
    ) -> Result<Vec<(AssetId, AssetValue, AssetAction)>, rocksdb::Error> {
        if let Some(asset_value_birth_pk_bytes) = self.storage.db.get_cf(
            &self.storage.families.custom.assets_by_utxo_pk_cf,
            birth_pk_bytes,
        )? {
            eutxo_codec_utxo::get_asset_value_ation_birth_pks(&asset_value_birth_pk_bytes)
                .iter()
                .map(|(asset_value, asset_action, birth_pk)| {
                    let asset_id = self
                        .storage
                        .db
                        .get_cf(
                            &self.storage.families.custom.asset_id_by_asset_birth_pk_cf,
                            birth_pk,
                        )?
                        .unwrap();
                    Ok((asset_id.into(), *asset_value, *asset_action))
                })
                .collect::<Result<Vec<(AssetId, AssetValue, AssetAction)>, rocksdb::Error>>()
        } else {
            Ok(vec![])
        }
    }

    fn get_o2m_utxo_indexes(
        &self,
        o2m_index_pks: &[(DbIndexNumber, UtxoBirthPkBytes)],
    ) -> Result<Vec<(DbIndexNumber, O2mIndexValue)>, rocksdb::Error> {
        o2m_index_pks
            .iter()
            .map(|(cf_index, utxo_birth_pk)| {
                let index_value = self
                    .storage
                    .db
                    .get_cf(
                        &self.storage.families.custom.o2m_index_by_utxo_birth_pk_cf[cf_index],
                        utxo_birth_pk,
                    )?
                    .unwrap();

                Ok((*cf_index, index_value.into()))
            })
            .collect::<Result<Vec<(DbIndexNumber, O2mIndexValue)>, rocksdb::Error>>()
    }

    fn get_outputs(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
    ) -> Result<Vec<EuUtxo>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        self.storage
            .db
            .prefix_iterator_cf(&self.storage.families.custom.utxo_value_by_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(utxo_pk, utxo_value_bytes)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let (utxo_value, o2m_index_pks, o2o_db_indexes) =
                        eutxo_codec_utxo::bytes_to_utxo(&utxo_value_bytes);

                    let o2m_db_indexes: Vec<(DbIndexNumber, O2mIndexValue)> =
                        self.get_o2m_utxo_indexes(&o2m_index_pks)?;

                    let assets: Vec<(AssetId, AssetValue, AssetAction)> =
                        self.get_assets(&utxo_pk)?;

                    Ok(EuUtxo {
                        utxo_index,
                        o2m_db_indexes,
                        o2o_db_indexes,
                        assets,
                        utxo_value,
                    })
                })
            })
            .collect()
    }

    fn get_tx_inputs(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
    ) -> Result<Vec<EuTxInput>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        self.storage
            .db
            .prefix_iterator_cf(
                &self.storage.families.custom.utxo_pk_by_input_pk_cf,
                pk_bytes,
            )
            .map(|result| {
                result.and_then(|(_, utxo_pk)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let tx_pk = eutxo_codec_utxo::tx_pk_from_utxo_pk(&utxo_pk);
                    let tx_hash_bytes = self
                        .storage
                        .db
                        .get_cf(&self.storage.families.shared.tx_hash_by_pk_cf, tx_pk)?
                        .unwrap();
                    let tx_hash = codec_tx::hash_bytes_to_tx_hash(&tx_hash_bytes);
                    Ok(EuTxInput::TxHashInput(TxHashWithIndex {
                        // TODO we are not returning OutputIndexInput here
                        tx_hash,
                        utxo_index,
                    }))
                })
            })
            .collect()
    }
}

impl TxReadService for EuTxReadService {
    type CF = EutxoFamilies;
    type Tx = EuTx;

    fn get_txs_by_height(&self, block_height: &BlockHeight) -> Result<Vec<EuTx>, ServiceError> {
        let height_bytes = codec_block::block_height_to_bytes(block_height);
        self.storage
            .db
            .prefix_iterator_cf(&self.storage.families.shared.tx_hash_by_pk_cf, height_bytes)
            .map(|result| {
                result
                    .and_then(|(tx_pk, tx_hash)| {
                        let tx_index = codec_tx::pk_bytes_to_tx_index(&tx_pk);
                        let tx_hash: TxHash = codec_tx::hash_bytes_to_tx_hash(&tx_hash);
                        let tx_outputs = self.get_outputs(block_height, &tx_index)?;
                        let tx_inputs = self.get_tx_inputs(block_height, &tx_index)?;
                        Ok(EuTx {
                            tx_hash,
                            tx_index,
                            tx_inputs,
                            tx_outputs,
                        })
                    })
                    .map_err(|err| ServiceError::new(&err.to_string()))
            })
            .collect()
    }
}
