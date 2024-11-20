use super::{
    eutxo_codec_utxo::{
        self, AssetBirthPkBytes, AssetValueActionBirthPk, UtxoBirthPkBytes, UtxoPkBytes,
        UtxoValueWithIndexes,
    },
    eutxo_families::EutxoFamilies,
    eutxo_model::{EuTxInput, EuUtxo, UtxoValue},
};
use crate::codec::EncodeDecode;
use crate::model::{
    AssetAction, AssetId, AssetMinted, AssetValue, Block, BlockHeight, DbIndexNumber,
    DbIndexValueSize, O2mIndexValue, O2oIndexValue, TxHash, TxPk, UtxoPk,
};
use crate::{
    api::TxWriteService, codec_tx::TxPkBytes, eutxo::eutxo_model::EuTx, rocks_db_batch::Families,
};

use byteorder::{BigEndian, ByteOrder};
use core::fmt::Debug;
use lru::LruCache;
use rocksdb::{
    BoundColumnFamily, MultiThreaded, OptimisticTransactionDB, WriteBatchWithTransaction,
};
use std::{mem::size_of, sync::Arc};

pub struct EuTxWriteService {
    pub perist_coinbase_inputs: bool,
}

impl EuTxWriteService {
    pub fn new(perist_coinbase_inputs: bool) -> Self {
        EuTxWriteService {
            perist_coinbase_inputs,
        }
    }

    // Method to process the outputs of a transaction
    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        for utxo in tx.tx_outputs.iter() {
            let utxo_pk_bytes =
                eutxo_codec_utxo::utxo_pk_bytes(block_height, &tx.tx_index, &utxo.utxo_index.0);

            self.persist_utxo(
                &utxo_pk_bytes,
                utxo,
                db_tx,
                batch,
                utxo_birth_pk_by_index_cache,
                families,
            )?;
            self.persist_assets(
                &utxo_pk_bytes,
                utxo,
                db_tx,
                batch,
                asset_birth_pk_by_asset_id_cache,
                families,
            )?;
        }
        Ok(())
    }

    fn remove_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<EutxoFamilies>,
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
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<EutxoFamilies>,
    ) {
        for (input_index, input) in tx.tx_inputs.iter().enumerate() {
            let utxo_pk_opt: Option<[u8; 8]> = match input {
                EuTxInput::TxHashInput(tx_input) => tx_pk_by_tx_hash_cache
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
                EuTxInput::OutputIndexInput(index_number, output_index) => utxo_pk_by_index_cache
                    .get(output_index)
                    .copied()
                    .or_else(|| {
                        let pk: Option<[u8; 8]> = db_tx
                            .get_cf(
                                &families.custom.o2o_utxo_birth_pk_by_index_cf[index_number],
                                &output_index.0,
                            )
                            .unwrap()
                            .map(|bytes| bytes.try_into().unwrap());
                        pk
                    }),
            };
            match utxo_pk_opt {
                Some(utxo_pk) => {
                    let input_pk = eutxo_codec_utxo::utxo_pk_bytes(
                        block_height,
                        &tx.tx_index,
                        &(input_index as u16),
                    );
                    batch.put_cf(&families.custom.utxo_pk_by_input_pk_cf, input_pk, utxo_pk);
                    batch.put_cf(&families.custom.input_pk_by_utxo_pk_cf, utxo_pk, input_pk);
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
        families: &Families<EutxoFamilies>,
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
                        .get(output_index)
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

    fn persist_assets(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        if !utxo.assets.is_empty() {
            // start building the asset_value_action_indexes
            let asset_elem_size =
                size_of::<AssetValue>() + size_of::<AssetAction>() + size_of::<AssetBirthPkBytes>();
            let mut asset_value_action_birth_pk = vec![0u8; utxo.assets.len() * asset_elem_size];
            let mut idx = 0;

            for (asset_index, (asset_id, asset_value, asset_action)) in
                utxo.assets.iter().enumerate()
            {
                let asset_pk_bytes =
                    eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));

                BigEndian::write_u64(
                    &mut asset_value_action_birth_pk[idx..idx + size_of::<AssetValue>()],
                    *asset_value,
                );
                idx += size_of::<AssetValue>();

                asset_value_action_birth_pk.push((*asset_action).into());
                idx += size_of::<AssetAction>();

                let (asset_birth_pk_bytes, _) = self.persist_asset_birth_pk_or_relation_with_pk(
                    asset_id,
                    &asset_pk_bytes,
                    asset_birth_pk_by_asset_id_cache,
                    &families.custom.asset_birth_pk_by_asset_id_cf,
                    &families.custom.asset_birth_pk_relations_cf,
                    &families.custom.asset_id_by_asset_birth_pk_cf,
                    db_tx,
                    batch,
                )?;

                asset_value_action_birth_pk[idx..idx + size_of::<AssetBirthPkBytes>()]
                    .copy_from_slice(&asset_birth_pk_bytes);
                idx += size_of::<AssetBirthPkBytes>();
            }
            self.persist_asset_value_action_birth_pk(
                utxo_pk_bytes,
                &asset_value_action_birth_pk,
                batch,
                families,
            );
        }
        Ok(())
    }

    fn persist_utxo(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo: &EuUtxo,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        let o2m_index_elem_length = utxo.o2m_db_indexes.len()
            * (size_of::<DbIndexNumber>() + size_of::<UtxoBirthPkBytes>());
        let o2o_index_elem_length: usize = utxo
            .o2o_db_indexes
            .iter()
            .map(|(_, v)| size_of::<DbIndexNumber>() + size_of::<DbIndexValueSize>() + v.0.len())
            .sum();

        let mut utxo_value_with_indexes =
            vec![0u8; size_of::<UtxoValue>() + o2o_index_elem_length + o2m_index_elem_length];

        BigEndian::write_u64(
            &mut utxo_value_with_indexes[0..size_of::<UtxoValue>()],
            utxo.utxo_value.0,
        );
        let mut index = size_of::<UtxoValue>();

        for (index_number, index_value) in utxo.o2m_db_indexes.iter() {
            utxo_value_with_indexes[index] = *index_number;
            index += size_of::<DbIndexNumber>();
            let (utxo_birth_pk_bytes, _) = self.persist_birth_pk_or_relation_with_pk(
                index_value,
                utxo_pk_bytes,
                utxo_birth_pk_by_index_cache,
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

        for (index_number, index_value) in utxo.o2o_db_indexes.iter() {
            utxo_value_with_indexes[index] = *index_number;
            index += size_of::<DbIndexNumber>();
            // index value size

            BigEndian::write_u16(
                &mut utxo_value_with_indexes[index..index + size_of::<DbIndexValueSize>()],
                index_value.0.len() as DbIndexValueSize,
            );
            index += size_of::<DbIndexValueSize>();
            // index value
            utxo_value_with_indexes[index..index + index_value.0.len()]
                .copy_from_slice(&index_value.0);
            index += index_value.0.len();

            db_tx.put_cf(
                &families.custom.o2o_utxo_birth_pk_by_index_cf[index_number],
                &index_value.0,
                utxo_pk_bytes,
            )?;
        }

        self.persist_utxo_value_with_indexes(
            utxo_pk_bytes,
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
        families: &Families<EutxoFamilies>,
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
        birth_pk_by_index_cf: &Arc<BoundColumnFamily>,
        birth_pk_relations_cf: &Arc<BoundColumnFamily>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily>,
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
            .prefix_iterator_cf(birth_pk_relations_cf, &birth_pk)
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
                        db_tx.delete_cf(birth_pk_relations_cf, relation_to_delete)
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
        assets: &[(AssetId, AssetValue, AssetAction)],
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        for (asset_index, (asset_id, _, _)) in assets.iter().enumerate() {
            let asset_pk_bytes =
                eutxo_codec_utxo::asset_pk_bytes(utxo_pk_bytes, &(asset_index as u8));
            self.remove_o2m_indexed_entry(
                &asset_pk_bytes,
                &asset_id.0,
                &families.custom.asset_birth_pk_by_asset_id_cf,
                &families.custom.asset_birth_pk_relations_cf,
                &families.custom.asset_id_by_asset_birth_pk_cf,
                db_tx,
                eutxo_codec_utxo::get_asset_pk_from_relation,
            )?;
        }
        self.remove_asset_value_with_indexes(utxo_pk_bytes, db_tx, families)?;
        Ok(())
    }

    fn persist_utxo_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        utxo_value_with_indexes: &UtxoValueWithIndexes,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<EutxoFamilies>,
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
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&families.custom.utxo_value_by_pk_cf, utxo_pk_bytes)
    }

    fn remove_asset_value_with_indexes(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        db_tx.delete_cf(&families.custom.assets_by_utxo_pk_cf, utxo_pk_bytes)
    }

    fn persist_asset_value_action_birth_pk(
        &self,
        utxo_pk_bytes: &UtxoPkBytes,
        asset_value_action_birth_pk: &AssetValueActionBirthPk,
        batch: &mut WriteBatchWithTransaction<true>,
        families: &Families<EutxoFamilies>,
    ) {
        batch.put_cf(
            &families.custom.assets_by_utxo_pk_cf,
            utxo_pk_bytes,
            asset_value_action_birth_pk,
        );
    }

    fn persist_birth_pk_or_relation_with_pk(
        &self,
        index_value: &O2mIndexValue,
        pk_bytes: &[u8],
        cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        birth_pk_by_index_cf: &Arc<BoundColumnFamily>,
        birth_pk_relations_cf: &Arc<BoundColumnFamily>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(Vec<u8>, AssetMinted), rocksdb::Error> {
        if let Some(existing_birth_pk_vec) = cache.get(index_value) {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_relations_cf, &birth_pk_with_pk, vec![]);
            Ok((existing_birth_pk_vec.clone(), false))
        } else if let Some(existing_birth_pk_vec) =
            db_tx.get_cf(birth_pk_by_index_cf, &index_value.0)?
        {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(&existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_relations_cf, &birth_pk_with_pk, vec![]);
            Ok((existing_birth_pk_vec, false))
        } else {
            let pk_bytes_vec = pk_bytes.to_vec();
            db_tx.put_cf(birth_pk_by_index_cf, &index_value.0, pk_bytes)?;
            batch.put_cf(index_by_birth_pk_cf, pk_bytes, &index_value.0);
            cache.put(index_value.clone(), pk_bytes_vec.clone());
            Ok((pk_bytes_vec, true))
        }
    }

    fn persist_asset_birth_pk_or_relation_with_pk(
        &self,
        index_value: &AssetId,
        pk_bytes: &[u8],
        cache: &mut LruCache<AssetId, Vec<u8>>,
        birth_pk_by_index_cf: &Arc<BoundColumnFamily>,
        birth_pk_relations_cf: &Arc<BoundColumnFamily>,
        index_by_birth_pk_cf: &Arc<BoundColumnFamily>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
    ) -> Result<(Vec<u8>, AssetMinted), rocksdb::Error> {
        if let Some(existing_birth_pk_vec) = cache.get(index_value) {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_relations_cf, &birth_pk_with_pk, vec![]);
            Ok((existing_birth_pk_vec.clone(), false))
        } else if let Some(existing_birth_pk_vec) =
            db_tx.get_cf(birth_pk_by_index_cf, &index_value.0)?
        {
            let birth_pk_with_pk =
                eutxo_codec_utxo::concat_birth_pk_with_pk(&existing_birth_pk_vec, pk_bytes);
            batch.put_cf(birth_pk_relations_cf, &birth_pk_with_pk, vec![]);
            Ok((existing_birth_pk_vec, false))
        } else {
            let pk_bytes_vec = pk_bytes.to_vec();
            db_tx.put_cf(birth_pk_by_index_cf, &index_value.0, pk_bytes)?;
            batch.put_cf(index_by_birth_pk_cf, pk_bytes, &index_value.0);
            cache.put(index_value.clone(), pk_bytes_vec.clone());
            Ok((pk_bytes_vec, true))
        }
    }
}

impl TxWriteService for EuTxWriteService {
    type CF = EutxoFamilies;
    type Tx = EuTx;

    fn persist_txs(
        &self,
        block: &Block<EuTx>,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        utxo_birth_pk_by_index_cache: &mut LruCache<O2mIndexValue, Vec<u8>>,
        asset_birth_pk_by_asset_id_cache: &mut LruCache<AssetId, Vec<u8>>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        for tx in block.txs.iter() {
            self.persist_tx(
                &block.header.height,
                tx,
                db_tx,
                batch,
                tx_pk_by_tx_hash_cache,
                families,
            )?;

            self.persist_outputs(
                &block.header.height,
                tx,
                db_tx,
                batch,
                utxo_birth_pk_by_index_cache,
                asset_birth_pk_by_asset_id_cache,
                families,
            )?;
            if self.perist_coinbase_inputs {
                self.persist_inputs(
                    &block.header.height,
                    tx,
                    db_tx,
                    batch,
                    tx_pk_by_tx_hash_cache,
                    utxo_pk_by_index_cache,
                    families,
                );
            }
        }
        Ok(())
    }

    fn persist_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        batch: &mut WriteBatchWithTransaction<true>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes: [u8; 6] = TxPk {
            block_height: *block_height,
            tx_index: tx.tx_index,
        }
        .encode_to_array();
        batch.put_cf(&families.shared.tx_hash_by_pk_cf, tx_pk_bytes, tx.tx_hash);
        db_tx.put_cf(&families.shared.tx_pk_by_hash_cf, tx.tx_hash, tx_pk_bytes)?;
        tx_pk_by_tx_hash_cache.put(tx.tx_hash, tx_pk_bytes);
        Ok(())
    }

    fn remove_tx(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        db_tx: &rocksdb::Transaction<OptimisticTransactionDB<MultiThreaded>>,
        tx_pk_by_tx_hash_cache: &mut LruCache<TxHash, TxPkBytes>,
        utxo_pk_by_index_cache: &mut LruCache<O2oIndexValue, UtxoPkBytes>,
        families: &Families<EutxoFamilies>,
    ) -> Result<(), rocksdb::Error> {
        let tx_pk_bytes: [u8; 6] = TxPk {
            block_height: *block_height,
            tx_index: tx.tx_index,
        }
        .encode_to_array();
        if self.perist_coinbase_inputs {
            self.remove_inputs(
                block_height,
                tx,
                db_tx,
                tx_pk_by_tx_hash_cache,
                utxo_pk_by_index_cache,
                families,
            )?;
        }
        self.remove_outputs(block_height, tx, db_tx, families)?;

        db_tx.delete_cf(&families.shared.tx_hash_by_pk_cf, tx_pk_bytes)?;

        db_tx.delete_cf(&families.shared.tx_pk_by_hash_cf, tx.tx_hash)?;

        tx_pk_by_tx_hash_cache.pop(&tx.tx_hash);

        Ok(())
    }
}
