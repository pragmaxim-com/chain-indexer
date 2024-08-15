use super::{
    eutxo_codec_utxo::{
        self, AssetBirthPkBytes, AssetPkBytes, AssetValueWithIndex, UtxoBirthPkBytes, UtxoPkBytes,
        UtxoValueWithIndexes,
    },
    eutxo_families::EutxoFamilies,
    eutxo_model::{EuTxInput, EuUtxo, TxHashWithIndex},
    eutxo_schema::DbIndexNumber,
};
use crate::{
    api::TxService,
    codec_block,
    codec_tx::{self, TxPkBytes},
    eutxo::eutxo_model::EuTx,
    model::{
        AssetAction, AssetId, AssetMinted, AssetValue, Block, BlockHeight, O2mIndexValue,
        O2oIndexValue, Transaction, TxHash, TxIndex,
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

pub struct EuTxService {}

impl<'db> EuTxService {
    // Method to process the outputs of a transaction
    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.persist_utxo(&utxo_pk_bytes, utxo, db_tx, batch, families)?;
            self.persist_assets(&utxo_pk_bytes, utxo, db_tx, batch, families)?;
        }
        Ok(())
    }

    fn remove_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.remove_utxo_indexes(
                &utxo_pk_bytes,
                &utxo.o2o_db_indexes,
                &utxo.o2m_db_indexes,
                db_tx,
                families,
            )?;
            self.remove_assets(&utxo_pk_bytes, &utxo.assets, db_tx, families)?;
        }
        Ok(())
    }

    // Method to process the inputs of a transaction
    fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) {
        for (input_index, input) in tx.tx_inputs.iter().enumerate() {
            let utxo_pk_opt: Option<[u8; 8]> = match input {
                EuTxInput::TxHashInput(tx_input) => tx_pk_by_tx_hash_lru_cache
                    .get(&tx_input.tx_hash)
                    .map(|tx_pk_bytes| {
                        eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, &tx_input.utxo_index)
                    })
                    .or_else(|| {
                        db_tx
                            .get_cf(&families.shared.tx_pk_by_hash_cf, tx_input.tx_hash)
                            .unwrap()
                            .map(|tx_pk_bytes| {
                                eutxo_codec_utxo::utxo_pk_bytes_from(
                                    &tx_pk_bytes,
                                    &tx_input.utxo_index,
                                )
                            })
                    }),
                EuTxInput::OutputIndexInput(index_number, output_index) => {
                    utxo_pk_by_index_lru_cache
                        .get(&output_index)
                        .map(|&arr| arr)
                        .or_else(|| {
                            let pk: Option<[u8; 8]> = db_tx
                                .get_cf(
                                    &families.custom.o2o_utxo_birth_pk_by_index_cf[index_number],
                                    &output_index.0,
                                )
                                .unwrap()
                                .unwrap()
                                .try_into()
                                .ok();
                            pk
                        })
                }
            };
            match utxo_pk_opt {
                Some(utxo_pk) => {
                    let input_pk = eutxo_codec_utxo::utxo_pk_bytes(
                        block_height,
                        &tx.tx_index,
                        &(input_index as u16),
                    );
                    batch.put_cf(&families.custom.utxo_pk_by_input_pk_cf, &input_pk, utxo_pk);
                    batch.put_cf(&families.custom.input_pk_by_utxo_pk_cf, utxo_pk, &input_pk);
                }
                None => {
                    // println!("Genesis {}", output_index);
                }
            }
        }
    }

    fn remove_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for (input_index, input) in tx.tx_inputs.iter().enumerate() {
            let utxo_pk = match input {
                EuTxInput::TxHashInput(tx_input) => tx_pk_by_tx_hash_lru_cache
                    .get(&tx_input.tx_hash)
                    .map(|tx_pk_bytes| {
                        eutxo_codec_utxo::utxo_pk_bytes_from(tx_pk_bytes, &tx_input.utxo_index)
                    })
                    .or_else(|| {
                        db_tx
                            .get_cf(&families.shared.tx_pk_by_hash_cf, tx_input.tx_hash)
                            .unwrap()
                            .map(|tx_pk_bytes| {
                                eutxo_codec_utxo::utxo_pk_bytes_from(
                                    &tx_pk_bytes,
                                    &tx_input.utxo_index,
                                )
                            })
                    })
                    .unwrap()
                    .to_vec(),
                EuTxInput::OutputIndexInput(index_number, output_index) => {
                    utxo_pk_by_index_lru_cache
                        .get(&output_index)
                        .map(|o| o.to_vec())
                        .or_else(|| {
                            db_tx
                                .get_cf(
                                    &families.custom.o2o_utxo_birth_pk_by_index_cf[index_number],
                                    &output_index.0,
                                )
                                .unwrap()
                        })
                        .unwrap()
                }
            };
            let input_pk =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &(input_index as u16));
            db_tx.delete_cf(&families.custom.utxo_pk_by_input_pk_cf, input_pk)?;
            db_tx.delete_cf(&families.custom.input_pk_by_utxo_pk_cf, utxo_pk)?;
        }
        Ok(())
    }

    fn get_assets(
        &self,
        birth_pk_bytes: &[u8],
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<Vec<(AssetId, AssetValue, AssetAction)>, rocksdb::Error> {
        db_tx
            .prefix_iterator_cf(&families.custom.asset_by_asset_pk_cf, birth_pk_bytes)
            .map(|result| {
                result.and_then(|(_, asset_value_birth_pk_action_bytes)| {
                    let (asset_value, asset_birth_pk, asset_action) =
                        eutxo_codec_utxo::get_asset_value_birth_pk_action(
                            &asset_value_birth_pk_action_bytes,
                        );
                    let asset_id = db_tx
                        .get_cf(
                            &families.custom.asset_id_by_asset_birth_pk_cf,
                            asset_birth_pk,
                        )?
                        .unwrap();
                    Ok((asset_id, asset_value, asset_action))
                })
            })
            .collect()
    }

    fn get_o2m_utxo_indexes(
        &self,
        o2m_index_pks: &Vec<(DbIndexNumber, UtxoBirthPkBytes)>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<Vec<(DbIndexNumber, O2mIndexValue)>, rocksdb::Error> {
        o2m_index_pks
            .iter()
            .map(|(cf_index, utxo_birth_pk)| {
                let index_value = db_tx
                    .get_cf(
                        &families.custom.o2m_index_by_utxo_birth_pk_cf[cf_index],
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
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<Vec<EuUtxo>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        db_tx
            .prefix_iterator_cf(&families.custom.utxo_value_by_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(utxo_pk, utxo_value_bytes)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let (utxo_value, o2m_index_pks, o2o_db_indexes) =
                        eutxo_codec_utxo::bytes_to_utxo(&utxo_value_bytes);

                    let o2m_db_indexes: Vec<(DbIndexNumber, O2mIndexValue)> =
                        self.get_o2m_utxo_indexes(&o2m_index_pks, db_tx, families)?;

                    let assets: Vec<(AssetId, AssetValue, AssetAction)> =
                        self.get_assets(&utxo_pk, db_tx, families)?;

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
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<Vec<EuTxInput>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        db_tx
            .prefix_iterator_cf(&families.custom.utxo_pk_by_input_pk_cf, pk_bytes)
            .map(|result| {
                result.and_then(|(_, utxo_pk)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let tx_pk = eutxo_codec_utxo::tx_pk_from_utxo_pk(&utxo_pk);
                    let tx_hash_bytes = db_tx
                        .get_cf(&families.shared.tx_hash_by_pk_cf, tx_pk)?
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

    fn persist_assets(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        if utxo.assets.len() > 0 {
            for (asset_index, (asset_id, asset_value, asset_action)) in
                utxo.assets.iter().enumerate()
            {
                let asset_pk_bytes =
                    eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));

                let mut asset_value_birth_pk =
                    vec![0u8; size_of::<AssetValue>() + size_of::<AssetBirthPkBytes>()];
                // append indexes to utxo_value_with_indexes
                BigEndian::write_u64(
                    &mut asset_value_birth_pk[0..size_of::<AssetValue>()],
                    *asset_value,
                );

                let (asset_birth_pk_bytes, _) = self.persist_birth_pk_or_relation_with_pk(
                    asset_id,
                    &asset_pk_bytes,
                    &families.custom.asset_birth_pk_by_asset_id_cf,
                    &families.custom.asset_birth_pk_with_asset_pk_cf,
                    &families.custom.asset_id_by_asset_birth_pk_cf,
                    db_tx,
                    batch,
                )?;

                asset_value_birth_pk[size_of::<AssetValue>()
                    ..size_of::<AssetValue>() + size_of::<AssetBirthPkBytes>()]
                    .copy_from_slice(&asset_birth_pk_bytes);

                asset_value_birth_pk.push((*asset_action).into());

                self.persist_asset_value_with_index(
                    &asset_pk_bytes,
                    &asset_value_birth_pk,
                    batch,
                    families,
                );
            }
        }
        Ok(())
    }

    fn persist_utxo(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        // start building the utxo_value_with_indexes
        let o2m_index_elem_length = size_of::<DbIndexNumber>() + size_of::<UtxoBirthPkBytes>();
        let o2o_index_elem_length = size_of::<DbIndexNumber>() + size_of::<O2oIndexValue>();

        let mut utxo_value_with_indexes =
            vec![
                0u8;
                size_of::<u64>()
                    + (utxo.o2m_db_indexes.len() * o2m_index_elem_length)
                    + (utxo.o2o_db_indexes.len() * o2o_index_elem_length)
            ];
        BigEndian::write_u64(
            &mut utxo_value_with_indexes[0..size_of::<UtxoBirthPkBytes>()],
            utxo.utxo_value.0,
        );
        let mut index = size_of::<UtxoBirthPkBytes>();

        for (index_number, index_value) in utxo.o2m_db_indexes.iter() {
            utxo_value_with_indexes[index] = *index_number;
            index += size_of::<DbIndexNumber>();

            let (utxo_birth_pk_bytes, _) = self.persist_birth_pk_or_relation_with_pk(
                &index_value.0,
                utxo_pk_bytes,
                &families.custom.o2m_utxo_birth_pk_by_index_cf[index_number],
                &families.custom.o2m_utxo_birth_pk_relations_cf[index_number],
                &families.custom.o2m_index_by_utxo_birth_pk_cf[index_number],
                db_tx,
                batch,
            )?;
            utxo_value_with_indexes[index..index + size_of::<UtxoBirthPkBytes>()]
                .copy_from_slice(&utxo_birth_pk_bytes);
            index += size_of::<UtxoBirthPkBytes>();
        }

        for (cf_index, index_value) in utxo.o2o_db_indexes.iter() {
            // index number
            utxo_value_with_indexes[index] = *cf_index;
            index += size_of::<DbIndexNumber>();
            // index value size
            utxo_value_with_indexes.extend_from_slice(&(index_value.0.len() as u16).to_be_bytes());
            index += size_of::<u16>();
            // index value
            utxo_value_with_indexes[index..index + index_value.0.len()]
                .copy_from_slice(&index_value.0);
            index += index_value.0.len();

            db_tx.put_cf(
                &families.custom.o2o_utxo_birth_pk_by_index_cf[cf_index],
                &index_value.0,
                utxo_pk_bytes,
            )?;
        }

        self.persist_utxo_value_with_indexes(
            &utxo_pk_bytes,
            &utxo_value_with_indexes,
            batch,
            families,
        );
        Ok(())
    }

    fn remove_utxo_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        o2o_indexes: &Vec<(DbIndexNumber, O2oIndexValue)>,
        o2m_indexes: &Vec<(DbIndexNumber, O2mIndexValue)>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for (cf_index, index_value) in o2m_indexes {
            self.remove_o2m_indexed_entry(
                utxo_pk_bytes,
                &index_value.0,
                &families.custom.o2m_utxo_birth_pk_by_index_cf[cf_index],
                &families.custom.o2m_utxo_birth_pk_relations_cf[cf_index],
                &families.custom.o2m_index_by_utxo_birth_pk_cf[cf_index],
                db_tx,
                eutxo_codec_utxo::get_utxo_pk_from_relation,
            )?;
        }
        for (cf_index, index_value) in o2o_indexes {
            db_tx.delete_cf(
                &families.custom.o2o_utxo_birth_pk_by_index_cf[cf_index],
                &index_value.0,
            )?;
        }

        self.remove_utxo_value_with_indexes(utxo_pk_bytes, db_tx, families)?;
        Ok(())
    }

    fn remove_o2m_indexed_entry<const N: usize, F>(
        &self,
        pk_bytes: &[u8; N],
        index_value: &[u8],
        birth_pk_by_index_cf: &Arc<BoundColumnFamily<'db>>,
        birth_pk_with_pk_cf: &Arc<BoundColumnFamily<'db>>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily<'db>>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
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
        assets: &Vec<(AssetId, AssetValue, AssetAction)>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for (asset_index, (asset_id, _, _)) in assets.iter().enumerate() {
            let asset_pk_bytes =
                eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));
            self.remove_o2m_indexed_entry(
                &asset_pk_bytes,
                &asset_id,
                &families.custom.asset_birth_pk_by_asset_id_cf,
                &families.custom.asset_birth_pk_with_asset_pk_cf,
                &families.custom.asset_id_by_asset_birth_pk_cf,
                db_tx,
                eutxo_codec_utxo::get_asset_pk_from_relation,
            )?;
            self.remove_asset_value_with_indexes(&asset_pk_bytes, db_tx, families)?;
        }
        Ok(())
    }

    fn persist_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo_value_with_indexes: &UtxoValueWithIndexes,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) {
        batch.put_cf(
            &families.custom.utxo_value_by_pk_cf,
            utxo_pk_bytes,
            utxo_value_with_indexes,
        );
    }

    fn remove_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&families.custom.utxo_value_by_pk_cf, utxo_pk_bytes)
    }

    fn remove_asset_value_with_indexes(
        &self,
        asset_pk_bytes: &AssetPkBytes,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&families.custom.asset_by_asset_pk_cf, asset_pk_bytes)
    }

    fn persist_asset_value_with_index(
        &self,
        asset_pk_bytes: &AssetPkBytes,
        asset_value_with_index: &AssetValueWithIndex,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) {
        batch.put_cf(
            &families.custom.asset_by_asset_pk_cf,
            asset_pk_bytes,
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
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(Vec<u8>, AssetMinted), rocksdb::Error> {
        if let Some(existing_birth_pk_vec) = db_tx.get_cf(birth_pk_by_index_cf, index_value)? {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(&existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_with_pk_cf, &birth_pk_with_pk, vec![]);
            Ok((existing_birth_pk_vec, false))
        } else {
            db_tx.put_cf(birth_pk_by_index_cf, index_value, pk_bytes)?;
            batch.put_cf(index_by_birth_pk_cf, pk_bytes, index_value);
            Ok((pk_bytes.to_vec(), true))
        }
    }
}

impl<'db> TxService<'db> for EuTxService {
    type CF = EutxoFamilies<'db>;
    type Tx = EuTx;

    fn get_txs_by_height(
        &self,
        block_height: &BlockHeight,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<Vec<EuTx>, rocksdb::Error> {
        let height_bytes = codec_block::block_height_to_bytes(&block_height);
        db_tx
            .prefix_iterator_cf(&families.shared.tx_hash_by_pk_cf, height_bytes)
            .map(|result| {
                result.and_then(|(tx_pk, tx_hash)| {
                    let tx_index = codec_tx::pk_bytes_to_tx_index(&tx_pk);
                    let tx_hash: TxHash = codec_tx::hash_bytes_to_tx_hash(&tx_hash);
                    let tx_outputs = self.get_outputs(block_height, &tx_index, db_tx, families)?;
                    let tx_inputs = self.get_tx_inputs(block_height, &tx_index, db_tx, families)?;
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
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        for tx in block.txs.iter() {
            self.persist_tx(
                &block.header.height,
                tx,
                db_tx,
                batch,
                tx_pk_by_tx_hash_lru_cache,
                utxo_pk_by_index_lru_cache,
                families,
            )?;
        }
        Ok(())
    }

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.tx_index);
        batch.put_cf(&families.shared.tx_hash_by_pk_cf, &tx_pk_bytes, &tx.tx_hash);

        db_tx.put_cf(&families.shared.tx_pk_by_hash_cf, &tx.tx_hash, &tx_pk_bytes)?;

        tx_pk_by_tx_hash_lru_cache.put(tx.tx_hash, tx_pk_bytes);

        self.persist_outputs(block_height, tx, db_tx, batch, families)?;
        if !tx.is_coinbase() {
            self.persist_inputs(
                block_height,
                tx,
                db_tx,
                batch,
                tx_pk_by_tx_hash_lru_cache,
                utxo_pk_by_index_lru_cache,
                families,
            );
        }
        Ok(())
    }

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_lru_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<'db, EutxoFamilies<'db>>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.tx_index);

        if !tx.is_coinbase() {
            self.remove_inputs(
                block_height,
                tx,
                db_tx,
                tx_pk_by_tx_hash_lru_cache,
                utxo_pk_by_index_lru_cache,
                families,
            )?;
        }
        self.remove_outputs(block_height, tx, db_tx, families)?;

        db_tx.delete_cf(&families.shared.tx_hash_by_pk_cf, tx_pk_bytes)?;

        db_tx.delete_cf(&families.shared.tx_pk_by_hash_cf, &tx.tx_hash)?;

        tx_pk_by_tx_hash_lru_cache.pop(&tx.tx_hash);

        Ok(())
    }
}
