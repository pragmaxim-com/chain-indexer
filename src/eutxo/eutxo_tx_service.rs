use super::{
    eutxo_codec_utxo::{
        self, AssetBirthPkBytes, AssetValueWithIndex, UtxoBirthPkBytes, UtxoPkBytes,
        UtxoValueWithIndexes,
    },
    eutxo_families::EutxoFamilies,
    eutxo_model::{EuTxInput, EuUtxo},
};
use crate::{
    api::TxService,
    codec_block,
    codec_tx::{self, TxPkBytes},
    eutxo::eutxo_model::EuTx,
    model::{
        AssetId, AssetValue, Block, BlockHeight, DbIndexCfIndex, DbIndexValue, Transaction, TxHash,
        TxIndex,
    },
    rocks_db_batch::Families,
};
use byteorder::{BigEndian, ByteOrder};
use core::fmt::Debug;
use lru::LruCache;
use rocksdb::{
    BoundColumnFamily, MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction,
};
use std::{mem::size_of, sync::Arc};

pub struct EuTxService<'db> {
    families: &'db Families<'db, EutxoFamilies<'db>>,
}

impl<'db> EuTxService<'db> {
    pub fn new(families: &'db Families<'db, EutxoFamilies<'db>>) -> Self {
        EuTxService { families }
    }
    // Method to process the outputs of a transaction
    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.persist_utxo(&utxo_pk_bytes, utxo, db_tx, batch)?;
            self.persist_assets(&utxo_pk_bytes, utxo, db_tx, batch)?;
        }
        Ok(())
    }

    fn remove_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.remove_utxo_indexes(&utxo_pk_bytes, &utxo.db_indexes, db_tx)?;
            self.remove_assets(&utxo_pk_bytes, &utxo.assets, db_tx)?;
        }
        Ok(())
    }

    // Method to process the inputs of a transaction
    fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) {
        for (input_index, tx_input) in tx.tx_inputs.iter().enumerate() {
            let utxo_pk = tx_pk_by_tx_hash_lru_cache
                .get(&tx_input.tx_hash)
                .map(|tx_pk_bytes| {
                    eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, &tx_input.utxo_index)
                })
                .or_else(|| {
                    db_tx
                        .get_cf(&self.families.shared.tx_pk_by_hash_cf, tx_input.tx_hash)
                        .unwrap()
                        .map(|tx_pk_bytes| {
                            eutxo_codec_utxo::utxo_pk_bytes_from(&tx_pk_bytes, &tx_input.utxo_index)
                        })
                })
                .unwrap();

            let input_pk =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &(input_index as u16));
            batch.put_cf(
                &self.families.custom.utxo_pk_by_input_pk_cf,
                &input_pk,
                &utxo_pk,
            )
        }
    }

    fn remove_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        for (input_index, _) in tx.tx_inputs.iter().enumerate() {
            let input_pk =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &(input_index as u16));
            db_tx.delete_cf(&self.families.custom.utxo_pk_by_input_pk_cf, input_pk)?;
        }
        Ok(())
    }

    fn get_assets(
        &self,
        birth_pk_bytes: &[u8],
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<Vec<(AssetId, AssetValue)>, rocksdb::Error> {
        if let Some(asset_value_birth_pk_bytes) =
            db_tx.get_cf(&self.families.custom.assets_by_utxo_pk_cf, birth_pk_bytes)?
        {
            eutxo_codec_utxo::get_asset_value_and_birth_pks(&asset_value_birth_pk_bytes)
                .iter()
                .map(|(asset_value, birth_pk)| {
                    let asset_id = db_tx
                        .get_cf(
                            &self.families.custom.asset_id_by_asset_birth_pk_cf,
                            birth_pk,
                        )?
                        .unwrap();
                    Ok((asset_id, *asset_value))
                })
                .collect::<Result<Vec<(AssetId, AssetValue)>, rocksdb::Error>>()
        } else {
            Ok(vec![])
        }
    }

    fn get_utxo_indexes(
        &self,
        index_utxo_birth_pk_by_cf_index: &Vec<(DbIndexCfIndex, UtxoBirthPkBytes)>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<Vec<(DbIndexCfIndex, DbIndexValue)>, rocksdb::Error> {
        index_utxo_birth_pk_by_cf_index
            .iter()
            .map(|(cf_index, utxo_birth_pk)| {
                let index_value = db_tx
                    .get_cf(
                        &self.families.custom.index_by_utxo_birth_pk_cf[*cf_index as usize],
                        utxo_birth_pk,
                    )?
                    .unwrap();

                Ok((*cf_index, index_value))
            })
            .collect::<Result<Vec<(DbIndexCfIndex, DbIndexValue)>, rocksdb::Error>>()
    }

    fn get_outputs(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<Vec<EuUtxo>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        db_tx
            .prefix_iterator_cf(&self.families.custom.utxo_value_by_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(utxo_pk, utxo_value_bytes)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let (utxo_value, index_utxo_birth_pk_by_cf_index) =
                        eutxo_codec_utxo::bytes_to_utxo(&utxo_value_bytes);

                    let db_indexes: Vec<(DbIndexCfIndex, DbIndexValue)> =
                        self.get_utxo_indexes(&index_utxo_birth_pk_by_cf_index, db_tx)?;

                    let assets: Vec<(AssetId, AssetValue)> = self.get_assets(&utxo_pk, db_tx)?;

                    Ok(EuUtxo {
                        utxo_index,
                        db_indexes,
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
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<Vec<EuTxInput>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        db_tx
            .prefix_iterator_cf(&self.families.custom.utxo_pk_by_input_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(_, utxo_pk)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let tx_pk = eutxo_codec_utxo::tx_pk_from_utxo_pk(&utxo_pk);
                    let tx_hash_bytes = db_tx
                        .get_cf(&self.families.shared.tx_hash_by_pk_cf, tx_pk)?
                        .unwrap();
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
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(), rocksdb::Error> {
        if utxo.assets.len() > 0 {
            // start building the utxo_value_with_indexes
            let asset_elem_size = size_of::<AssetValue>() + size_of::<AssetBirthPkBytes>();
            let mut asset_value_birth_pk = vec![0u8; utxo.db_indexes.len() * asset_elem_size];
            let mut idx = 0;

            for (asset_index, (asset_id, asset_value)) in utxo.assets.iter().enumerate() {
                let asset_pk_bytes =
                    eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));

                // append indexes to utxo_value_with_indexes
                BigEndian::write_u64(
                    &mut asset_value_birth_pk[idx..idx + size_of::<AssetValue>()],
                    *asset_value,
                );
                idx += size_of::<AssetValue>();

                let asset_birth_pk_bytes: Vec<u8> = self.persist_birth_pk_or_relation_with_pk(
                    asset_id,
                    &asset_pk_bytes,
                    &self.families.custom.asset_birth_pk_by_asset_id_cf,
                    &self.families.custom.asset_birth_pk_with_asset_pk_cf,
                    &self.families.custom.asset_id_by_asset_birth_pk_cf,
                    db_tx,
                    batch,
                )?;

                asset_value_birth_pk[idx..idx + size_of::<AssetBirthPkBytes>()]
                    .copy_from_slice(&asset_birth_pk_bytes);
                idx += size_of::<AssetBirthPkBytes>();
            }
            self.persist_asset_value_with_index(&utxo_pk_bytes, &asset_value_birth_pk, batch);
        }
        Ok(())
    }

    fn persist_utxo(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(), rocksdb::Error> {
        // start building the utxo_value_with_indexes
        let index_elem_length = size_of::<DbIndexCfIndex>() + size_of::<UtxoBirthPkBytes>();

        let mut utxo_value_with_indexes =
            vec![0u8; size_of::<u64>() + (utxo.db_indexes.len() * index_elem_length)];
        BigEndian::write_u64(
            &mut utxo_value_with_indexes[0..size_of::<UtxoBirthPkBytes>()],
            utxo.utxo_value.0,
        );
        let mut index = size_of::<UtxoBirthPkBytes>();

        for (cf_index, index_value) in utxo.db_indexes.iter() {
            utxo_value_with_indexes[index] = *cf_index;
            index += size_of::<DbIndexCfIndex>();

            let utxo_birth_pk_bytes: Vec<u8> = self.persist_birth_pk_or_relation_with_pk(
                index_value,
                utxo_pk_bytes,
                &self.families.custom.utxo_birth_pk_by_index_cf[*cf_index as usize],
                &self.families.custom.utxo_birth_pk_with_utxo_pk_cf[*cf_index as usize],
                &self.families.custom.index_by_utxo_birth_pk_cf[*cf_index as usize],
                db_tx,
                batch,
            )?;
            utxo_value_with_indexes[index..index + size_of::<UtxoBirthPkBytes>()]
                .copy_from_slice(&utxo_birth_pk_bytes);
            index += size_of::<UtxoBirthPkBytes>();
        }

        self.persist_utxo_value_with_indexes(&utxo_pk_bytes, &utxo_value_with_indexes, batch);
        Ok(())
    }

    fn remove_utxo_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        db_indexes: &Vec<(DbIndexCfIndex, DbIndexValue)>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        for (cf_index, index_value) in db_indexes {
            self.remove_indexed_entry(
                utxo_pk_bytes,
                index_value,
                &self.families.custom.utxo_birth_pk_by_index_cf[*cf_index as usize],
                &self.families.custom.utxo_birth_pk_with_utxo_pk_cf[*cf_index as usize],
                &self.families.custom.index_by_utxo_birth_pk_cf[*cf_index as usize],
                db_tx,
                eutxo_codec_utxo::get_utxo_pk_from_relation,
            )?;
        }
        self.remove_utxo_value_with_indexes(utxo_pk_bytes, db_tx)?;
        Ok(())
    }

    fn remove_indexed_entry<const N: usize, F>(
        &self,
        pk_bytes: &[u8; N],
        index_value: &[u8],
        birth_pk_by_index_cf: &Arc<BoundColumnFamily<'db>>,
        birth_pk_with_pk_cf: &Arc<BoundColumnFamily<'db>>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily<'db>>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        mut f: F,
    ) -> Result<(), rocksdb::Error>
    where
        F: FnMut(&[u8]) -> [u8; N],
        [u8; N]: Debug,
    {
        let birth_pk = db_tx.get_cf(birth_pk_by_index_cf, index_value)?.unwrap();
        // find and remove relations if any, if there are no relations yet, it was a new index and we delete it
        let mut relations_counter = 0;
        db_tx
            .prefix_iterator_cf(birth_pk_with_pk_cf, &birth_pk)
            .filter_map(|result| match result {
                Ok((relation, _)) => {
                    relations_counter += 1;
                    let pk = f(&relation);
                    if pk == *pk_bytes {
                        Some(Ok(relation.to_vec()))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<Vec<u8>>, rocksdb::Error>>()
            .and_then(|relations_to_delete| {
                relations_to_delete
                    .iter()
                    .map(|relation_to_delete| {
                        db_tx.delete_cf(birth_pk_with_pk_cf, relation_to_delete)
                    })
                    .collect::<Result<Vec<()>, rocksdb::Error>>()
            })?;
        if relations_counter == 0 {
            db_tx.delete_cf(index_by_birth_pk_cf, &birth_pk)?;
            db_tx.delete_cf(birth_pk_by_index_cf, index_value)?;
        }
        Ok(())
    }

    fn remove_assets(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        assets: &Vec<(AssetId, AssetValue)>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        for (asset_index, (_, asset_value)) in assets.iter().enumerate() {
            let asset_pk_bytes =
                eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));
            self.remove_indexed_entry(
                &asset_pk_bytes,
                &asset_value.to_be_bytes(),
                &self.families.custom.asset_birth_pk_by_asset_id_cf,
                &self.families.custom.asset_birth_pk_with_asset_pk_cf,
                &self.families.custom.asset_id_by_asset_birth_pk_cf,
                db_tx,
                eutxo_codec_utxo::get_asset_pk_from_relation,
            )?;
        }
        self.remove_asset_value_with_indexes(utxo_pk_bytes, db_tx)?;
        Ok(())
    }

    fn persist_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo_value_with_indexes: &UtxoValueWithIndexes,
        batch: &mut WriteBatchWithTransaction<true>,
    ) {
        batch.put_cf(
            &self.families.custom.utxo_value_by_pk_cf,
            utxo_pk_bytes,
            utxo_value_with_indexes,
        );
    }

    fn remove_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&self.families.custom.utxo_value_by_pk_cf, utxo_pk_bytes)
    }

    fn remove_asset_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&self.families.custom.assets_by_utxo_pk_cf, utxo_pk_bytes)
    }

    fn persist_asset_value_with_index(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        asset_value_with_index: &AssetValueWithIndex,
        batch: &mut WriteBatchWithTransaction<true>,
    ) {
        batch.put_cf(
            &self.families.custom.assets_by_utxo_pk_cf,
            utxo_pk_bytes,
            asset_value_with_index,
        );
    }

    fn persist_birth_pk_or_relation_with_pk(
        &self,
        index_value: &[u8],
        pk_bytes: &[u8],
        birth_pk_by_index_cf: &Arc<BoundColumnFamily<'db>>,
        birth_pk_with_pk_cf: &Arc<BoundColumnFamily<'db>>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily<'db>>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<Vec<u8>, rocksdb::Error> {
        if let Some(existing_birth_pk_vec) = db_tx.get_cf(birth_pk_by_index_cf, index_value)? {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(&existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_with_pk_cf, &birth_pk_with_pk, vec![]);
            Ok(existing_birth_pk_vec)
        } else {
            db_tx.put_cf(birth_pk_by_index_cf, index_value, pk_bytes)?;
            batch.put_cf(index_by_birth_pk_cf, pk_bytes, index_value);
            Ok(pk_bytes.to_vec())
        }
    }
}

impl<'db> TxService<'db> for EuTxService<'db> {
    type CF = EutxoFamilies<'db>;
    type Tx = EuTx;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
    ) -> Result<Vec<EuTx>, rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        db_tx
            .prefix_iterator_cf(&self.families.shared.tx_hash_by_pk_cf, height_bytes)
            .map(|result| {
                result.and_then(|(tx_pk, tx_hash)| {
                    let tx_index = codec_tx::pk_bytes_to_tx_index(&tx_pk);
                    let tx_hash: TxHash = codec_tx::hash_bytes_to_tx_hash(&tx_hash);
                    let tx_outputs = self.get_outputs(block_height, &tx_index, db_tx)?;
                    let tx_inputs = self.get_tx_inputs(block_height, &tx_index, db_tx)?;
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

    fn persist_txs(
        &self,
        block: &Block<EuTx>,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error> {
        for tx in block.txs.iter() {
            self.persist_tx(
                &block.header.height,
                tx,
                db_tx,
                batch,
                tx_pk_by_tx_hash_lru_cache,
            )?;
        }
        Ok(())
    }

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.tx_index);
        batch.put_cf(
            &self.families.shared.tx_hash_by_pk_cf,
            &tx_pk_bytes,
            &tx.tx_hash,
        );

        db_tx.put_cf(
            &self.families.shared.tx_pk_by_hash_cf,
            &tx.tx_hash,
            &tx_pk_bytes,
        )?;

        tx_pk_by_tx_hash_lru_cache.put(tx.tx_hash, tx_pk_bytes);

        self.persist_outputs(block_height, tx, db_tx, batch)?;
        if !tx.is_coinbase() {
            self.persist_inputs(block_height, tx, db_tx, batch, tx_pk_by_tx_hash_lru_cache);
        }
        Ok(())
    }

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<'db, OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.tx_index);

        if !tx.is_coinbase() {
            self.remove_inputs(block_height, tx, db_tx)?;
        }
        self.remove_outputs(block_height, tx, db_tx)?;

        db_tx.delete_cf(&self.families.shared.tx_hash_by_pk_cf, tx_pk_bytes)?;

        db_tx.delete_cf(&self.families.shared.tx_pk_by_hash_cf, &tx.tx_hash)?;

        tx_pk_by_tx_hash_lru_cache.pop(&tx.tx_hash);

        Ok(())
    }
}
