use crate::{
    api::TxService,
    codec_block,
    codec_tx::{self, TxPkBytes},
    eutxo::eutxo_model::EuTx,
    indexer::RocksDbBatch,
    model::{BlockHeight, TxHash, TxIndex},
};
use lru::LruCache;
use std::cell::{RefCell, RefMut};

use super::{
    eutxo_codec_utxo,
    eutxo_model::{EuTxInput, UtxoIndex, UtxoValue},
};

pub struct EuTxService {}

impl EuTxService {
    fn get_utxo_value_by_index(
        &self,
        block_height: &BlockHeight,
        tx_index: &TxIndex,
        mut_batch: &mut RocksDbBatch,
    ) -> Result<Vec<(UtxoIndex, UtxoValue)>, rocksdb::Error> {
        let pk_bytes = codec_tx::tx_pk_bytes(block_height, tx_index);
        mut_batch
            .db_tx
            .prefix_iterator_cf(mut_batch.utxo_value_by_pk_cf, pk_bytes)
            .map(|result| {
                result.map(|(utxo_pk, utxo_value_bytes)| {
                    let utxo_index = eutxo_codec_utxo::utxo_index_from_pk_bytes(&utxo_pk);
                    let utxo_value = eutxo_codec_utxo::bytes_to_utxo_value(&utxo_value_bytes);
                    (utxo_index, utxo_value)
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
                result.map(|(tx_pk, tx_hash)| {
                    let tx_index = codec_tx::pk_bytes_to_tx_index(&tx_pk);
                    let tx_hash: TxHash = codec_tx::hash_bytes_to_tx_hash(&tx_hash);
                    let utxos =
                        self.get_utxo_value_by_index(block_height, &tx_index, &mut mut_batch)?;
                    let tx_inputs = self.get_tx_inputs(block_height, &tx_index, &mut mut_batch)?;
                    Ok(EuTx {
                        tx_hash,
                        tx_index,
                        tx_inputs,
                        utxos,
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
        let tx_pk_bytes = codec_tx::tx_pk_bytes(block_height, &tx.index);
        let tx_hash_by_pk_cf = batch.tx_hash_by_pk_cf;
        let tx_pk_by_hash_cf = batch.tx_pk_by_hash_cf;
        batch.batch.put_cf(tx_hash_by_pk_cf, tx_pk_bytes, &tx.hash);

        tx_pk_by_tx_hash_lru_cache.put(tx.hash, tx_pk_bytes);
        batch.db_tx.put_cf(tx_pk_by_hash_cf, &tx.hash, tx_pk_bytes)
    }

    // Method to process the outputs of a transaction
    fn persist_outputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
    ) {
        for utxo in tx.outs.iter() {
            let utxo_pk_bytes = eutxo_codec_utxo::pk_bytes(block_height, &tx.index, &utxo.index.0);
            let utxo_value_bytes = eutxo_codec_utxo::utxo_value_to_bytes(&utxo.value);
            let utxo_value_by_pk_cf = batch.utxo_value_by_pk_cf;
            batch
                .batch
                .put_cf(utxo_value_by_pk_cf, utxo_pk_bytes, utxo_value_bytes);

            for (db_index_name, db_index_value) in utxo.db_indexes.iter() {
                let utxo_index_cf = batch
                    .index_cf_by_name
                    .iter()
                    .find(|&i| db_index_name == &i.0)
                    .unwrap()
                    .1;
                batch
                    .batch
                    .merge_cf(utxo_index_cf, db_index_value, utxo_pk_bytes)
            }
        }
    }

    // Method to process the inputs of a transaction
    fn persist_inputs(
        &self,
        block_height: &BlockHeight,
        tx: &EuTx,
        batch: &mut RefMut<RocksDbBatch>,
        tx_pk_by_tx_hash_lru_cache: &mut LruCache<TxHash, TxPkBytes>,
    ) {
        for (input_index, tx_input) in tx.ins.iter().enumerate() {
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
                eutxo_codec_utxo::pk_bytes(block_height, &tx.index, &(input_index as u16));
            let utxo_pk_by_input_pk_cf = batch.utxo_pk_by_input_pk_cf;
            batch
                .batch
                .put_cf(utxo_pk_by_input_pk_cf, input_pk, utxo_pk)
        }
    }
}
