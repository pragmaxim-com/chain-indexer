use crate::{
    api::TxService,
    codec_block,
    codec_tx::{self, TxPkBytes},
    db_index_manager::DbIndexManager,
    eutxo::eutxo_model::EuTx,
    model::{AssetIndex, AssetValue, BlockHeight, DbIndexCfIndex, DbIndexValue, TxHash, TxIndex},
    rocks_db_batch::RocksDbBatch,
};
use byteorder::{BigEndian, ByteOrder};
use lru::LruCache;
use rocksdb::ColumnFamily;
use std::{
    cell::{RefCell, RefMut},
    mem::size_of,
    sync::Arc,
};

use super::{
    eutxo_codec_utxo::{
        self, AssetBirthPkBytes, UtxoBirthPkBytes, UtxoPkBytes, UtxoValueWithIndexes,
    },
    eutxo_model::{EuTxInput, EuUtxo},
};

pub struct EuTxService {
    pub db_index_manager: Arc<DbIndexManager>,
}

impl EuTxService {
    fn get_utxo_indexes(
        &self,
        index_utxo_birth_pk_by_cf_index: &Vec<(DbIndexCfIndex, UtxoBirthPkBytes)>,
        mut_batch: &mut RocksDbBatch,
    ) -> Result<Vec<(DbIndexCfIndex, DbIndexValue)>, rocksdb::Error> {
        index_utxo_birth_pk_by_cf_index
            .iter()
            .map(|(cf_index, utxo_birth_pk)| {
                let utxo_birth_pk_with_utxo_pk_cf =
                    mut_batch.utxo_birth_pk_with_utxo_pk_cf[*cf_index as usize];
                let index_by_utxo_birth_pk_cf =
                    mut_batch.index_by_utxo_birth_pk_cf[*cf_index as usize];
                let utxo_birth_pk_by_index_cf =
                    mut_batch.utxo_birth_pk_by_index_cf[*cf_index as usize];

                // get index value for this particular UtxoBirthPk

                let index_value = mut_batch
                    .db_tx
                    .get_cf(index_by_utxo_birth_pk_cf, utxo_birth_pk)?
                    .unwrap();

                Ok((*cf_index, index_value))
            })
            .collect::<Result<Vec<(DbIndexCfIndex, DbIndexValue)>, rocksdb::Error>>()
    }

    fn get_outputs(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
        mut_batch: &mut RocksDbBatch,
    ) -> Result<Vec<EuUtxo>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        mut_batch
            .db_tx
            .prefix_iterator_cf(mut_batch.utxo_value_by_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(utxo_pk, utxo_value_bytes)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let (utxo_value, index_utxo_birth_pk_by_cf_index) =
                        eutxo_codec_utxo::bytes_to_utxo(&utxo_value_bytes);

                    let db_indexes: Vec<(DbIndexCfIndex, DbIndexValue)> =
                        self.get_utxo_indexes(&index_utxo_birth_pk_by_cf_index, mut_batch)?;

                    Ok(EuUtxo {
                        utxo_index,
                        db_indexes,
                        assets: vec![], // todo
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
        mut_batch: &mut RocksDbBatch,
    ) -> Result<Vec<EuTxInput>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        let db_tx = mut_batch.db_tx;
        db_tx
            .prefix_iterator_cf(mut_batch.utxo_pk_by_input_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(_, utxo_pk)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let tx_pk = eutxo_codec_utxo::tx_pk_from_utxo_pk(&utxo_pk);
                    let tx_hash_bytes = db_tx.get_cf(mut_batch.tx_hash_by_pk_cf, tx_pk)?.unwrap();
                    let tx_hash = codec_tx::hash_bytes_to_tx_hash(&tx_hash_bytes);
                    Ok(EuTxInput {
                        tx_hash,
                        utxo_index,
                    })
                })
            })
            .collect()
    }

    fn persist_assets(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        batch: &mut RocksDbBatch,
    ) -> Result<(), rocksdb::Error> {
        let assets_by_utxo_pk_cf = batch.assets_by_utxo_pk_cf;
        let birth_pk_by_index_cf = batch.asset_birth_pk_by_asset_id_cf;
        let birth_pk_with_pk_cf = batch.asset_birth_pk_with_asset_pk_cf;
        let index_by_birth_pk_cf = batch.asset_id_by_asset_birth_pk_cf;

        // start building the utxo_value_with_indexes
        let mut aidx_value_birth_pk = vec![0u8; utxo.db_indexes.len() * 18];
        let mut index = 0;

        for (asset_index, (asset_id, asset_value)) in utxo.assets.iter().enumerate() {
            let asset_pk_bytes =
                eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));

            let asset_birth_pk_bytes: Vec<u8> = self.persist_birth_pk_or_relation_with_pk(
                asset_id,
                &asset_pk_bytes,
                birth_pk_by_index_cf,
                birth_pk_with_pk_cf,
                index_by_birth_pk_cf,
                batch,
            )?;

            // append indexes to utxo_value_with_indexes
            aidx_value_birth_pk[index] = asset_index as u8;
            index += size_of::<AssetIndex>();
            BigEndian::write_u64(&mut aidx_value_birth_pk[index..index + 8], *asset_value);
            index += size_of::<AssetValue>();
            aidx_value_birth_pk[index..index + 9].copy_from_slice(&asset_birth_pk_bytes);
            index += size_of::<AssetBirthPkBytes>();
        }

        self.persist_asset_value_with_index(&utxo_pk_bytes, &aidx_value_birth_pk, batch);
        Ok(())
    }

    fn persist_utxo(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        batch: &mut RocksDbBatch,
    ) -> Result<(), rocksdb::Error> {
        // start building the utxo_value_with_indexes
        let mut utxo_value_with_indexes = vec![0u8; 8 + utxo.db_indexes.len() * 9];
        BigEndian::write_u64(&mut utxo_value_with_indexes[0..8], utxo.utxo_value.0);
        let mut index = 8;

        for (cf_index, index_value) in utxo.db_indexes.iter() {
            let birth_pk_by_index_cf = batch.utxo_birth_pk_by_index_cf[*cf_index as usize];
            let birth_pk_with_pk_cf = batch.utxo_birth_pk_with_utxo_pk_cf[*cf_index as usize];
            let index_by_birth_pk_cf = batch.index_by_utxo_birth_pk_cf[*cf_index as usize];
            // first check if IndexValue has been already indexed or it is the first time
            let utxo_birth_pk_bytes: Vec<u8> = self.persist_birth_pk_or_relation_with_pk(
                index_value,
                utxo_pk_bytes,
                birth_pk_by_index_cf,
                birth_pk_with_pk_cf,
                index_by_birth_pk_cf,
                batch,
            )?;
            // append indexes to utxo_value_with_indexes
            utxo_value_with_indexes[index] = *cf_index;
            index += size_of::<DbIndexCfIndex>();
            utxo_value_with_indexes[index..index + size_of::<UtxoBirthPkBytes>()]
                .copy_from_slice(&utxo_birth_pk_bytes);
            index += size_of::<UtxoBirthPkBytes>();
        }

        self.persist_utxo_value_with_indexes(&utxo_pk_bytes, &utxo_value_with_indexes, batch);
        Ok(())
    }

    fn persist_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo_value_with_indexes: &UtxoValueWithIndexes,
        batch: &mut RocksDbBatch,
    ) {
        // persist utxo_value_with_indexes
        let utxo_value_by_pk_cf = batch.utxo_value_by_pk_cf;
        batch
            .batch
            .put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, &utxo_value_with_indexes);
    }

    fn persist_asset_value_with_index(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        asset_value_with_index: &UtxoValueWithIndexes,
        batch: &mut RocksDbBatch,
    ) {
        let utxo_value_by_pk_cf = batch.utxo_value_by_pk_cf;
        batch
            .batch
            .put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, &asset_value_with_index);
    }

    fn persist_birth_pk_or_relation_with_pk<'db>(
        &self,
        index_value: &[u8],
        pk_bytes: &[u8],
        birth_pk_by_index_cf: &'db ColumnFamily,
        birth_pk_with_pk_cf: &'db ColumnFamily,
        index_by_birth_pk_cf: &'db ColumnFamily,
        batch: &mut RocksDbBatch<'db>,
    ) -> Result<Vec<u8>, rocksdb::Error> {
        if let Some(existing_birth_pk_vec) =
            batch.db_tx.get_cf(birth_pk_by_index_cf, index_value)?
        {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(&existing_birth_pk_vec, pk_bytes);
            batch
                .batch
                .put_cf(birth_pk_with_pk_cf, birth_pk_with_pk, vec![]);
            Ok(existing_birth_pk_vec)
        } else {
            batch
                .db_tx
                .put_cf(birth_pk_by_index_cf, pk_bytes, index_value)?;
            batch
                .db_tx
                .put_cf(index_by_birth_pk_cf, index_value, pk_bytes)?;
            Ok(pk_bytes.to_vec())
        }
    }
}

impl TxService for EuTxService {
    type Tx = EuTx;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        batch: &RefCell<RocksDbBatch>,
    ) -> Result<Vec<EuTx>, rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        let mut mut_batch = batch.borrow_mut();

        mut_batch
            .db_tx
            .prefix_iterator_cf(mut_batch.tx_hash_by_pk_cf, height_bytes)
            .map(|result| {
                result.and_then(|(tx_pk, tx_hash)| {
                    let tx_index = codec_tx::pk_bytes_to_tx_index(&tx_pk);
                    let tx_hash: TxHash = codec_tx::hash_bytes_to_tx_hash(&tx_hash);
                    let tx_outputs = self.get_outputs(block_height, &tx_index, &mut mut_batch)?;
                    let tx_inputs = self.get_tx_inputs(block_height, &tx_index, &mut mut_batch)?;
                    Ok(EuTx {
                        tx_hash,
                        tx_index,
                        tx_inputs,
                        tx_outputs,
                    })
                })
            })
            .collect()
    }

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.tx_index);
        let tx_hash_by_pk_cf = batch.tx_hash_by_pk_cf;
        let tx_pk_by_hash_cf = batch.tx_pk_by_hash_cf;
        batch
            .batch
            .put_cf(tx_hash_by_pk_cf, tx_pk_bytes, &tx.tx_hash);

        tx_pk_by_tx_hash_lru_cache.put(tx.tx_hash, tx_pk_bytes);
        batch
            .db_tx
            .put_cf(tx_pk_by_hash_cf, &tx.tx_hash, tx_pk_bytes)
    }

    // Method to process the outputs of a transaction
    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.persist_utxo(&utxo_pk_bytes, utxo, batch)?;
            self.persist_assets(&utxo_pk_bytes, utxo, batch)?;
        }
        Ok(())
    }

    // Method to process the inputs of a transaction
    fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) {
        for (input_index, tx_input) in tx.tx_inputs.iter().enumerate() {
            let tx_pk_bytes = tx_pk_by_tx_hash_lru_cache
                .get(&tx_input.tx_hash)
                .map(|f| f.to_vec())
                .or(batch
                    .db_tx
                    .get_cf(batch.tx_pk_by_hash_cf, tx_input.tx_hash)
                    .unwrap())
                .unwrap();

            let utxo_pk = eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, &tx_input.utxo_index);
            let input_pk =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &(input_index as u16));
            let utxo_pk_by_input_pk_cf = batch.utxo_pk_by_input_pk_cf;
            batch
                .batch
                .put_cf(utxo_pk_by_input_pk_cf, input_pk, utxo_pk)
        }
    }
}
