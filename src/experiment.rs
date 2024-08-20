use rocksdb::{BoundColumnFamily, MultiThreaded, OptimisticTransactionDB};
use std::{collections::HashMap, mem::transmute, sync::Arc};

use crate::{
    eutxo::{
        eutxo_families::EutxoFamilies,
        eutxo_model::*,
        eutxo_schema::{DbIndexNumber, DbSchema},
    },
    model::*,
    rocks_db_batch::{CustomFamilies, Families, SharedFamilies},
};

pub struct Persistence<CF: CustomFamilies> {
    pub db: Arc<OptimisticTransactionDB<MultiThreaded>>,
    pub families: Arc<Families<CF>>,
}

impl Persistence<EutxoFamilies> {
    pub fn new(db: Arc<OptimisticTransactionDB<MultiThreaded>>, db_shema: &DbSchema) -> Self {
        Persistence {
            db: Arc::clone(&db),
            families: Arc::new(Families {
                shared: SharedFamilies {
                    meta_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(META_CF).unwrap(),
                        )
                    },
                    block_hash_by_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(BLOCK_PK_BY_HASH_CF).unwrap(),
                        )
                    },
                    block_pk_by_hash_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(BLOCK_HASH_BY_PK_CF).unwrap(),
                        )
                    },

                    tx_hash_by_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(TX_HASH_BY_PK_CF).unwrap(),
                        )
                    },

                    tx_pk_by_hash_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(TX_PK_BY_HASH_CF).unwrap(),
                        )
                    },
                },
                custom: EutxoFamilies {
                    utxo_value_by_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(UTXO_VALUE_BY_PK_CF).unwrap(),
                        )
                    },
                    utxo_pk_by_input_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(UTXO_PK_BY_INPUT_PK_CF).unwrap(),
                        )
                    },
                    input_pk_by_utxo_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(INPUT_PK_BY_UTXO_PK_CF).unwrap(),
                        )
                    },

                    o2m_utxo_birth_pk_relations_cf: db_shema
                        .o2m_index_name_by_number
                        .utxo_birth_pk_relations
                        .iter()
                        .map(|(index_number, index_name, _)| {
                            (*index_number, unsafe {
                                transmute::<_, Arc<BoundColumnFamily<'static>>>(
                                    db.cf_handle(index_name).unwrap(),
                                )
                            })
                        })
                        .collect::<HashMap<DbIndexNumber, Arc<BoundColumnFamily>>>(),
                    o2m_utxo_birth_pk_by_index_cf: db_shema
                        .o2m_index_name_by_number
                        .utxo_birth_pk_by_index
                        .iter()
                        .map(|(index_number, index_name, _)| {
                            (*index_number, unsafe {
                                transmute::<_, Arc<BoundColumnFamily<'static>>>(
                                    db.cf_handle(index_name).unwrap(),
                                )
                            })
                        })
                        .collect::<HashMap<DbIndexNumber, Arc<BoundColumnFamily>>>(),
                    o2m_index_by_utxo_birth_pk_cf: db_shema
                        .o2m_index_name_by_number
                        .index_by_utxo_birth_pk
                        .iter()
                        .map(|(index_number, index_name, _)| {
                            (*index_number, unsafe {
                                transmute::<_, Arc<BoundColumnFamily<'static>>>(
                                    db.cf_handle(index_name).unwrap(),
                                )
                            })
                        })
                        .collect::<HashMap<DbIndexNumber, Arc<BoundColumnFamily>>>(),
                    o2o_utxo_birth_pk_by_index_cf: db_shema
                        .o2o_index_name_by_number
                        .utxo_birth_pk_by_index
                        .iter()
                        .map(|(index_number, index_name, _)| {
                            (*index_number, unsafe {
                                transmute::<_, Arc<BoundColumnFamily<'static>>>(
                                    db.cf_handle(index_name).unwrap(),
                                )
                            })
                        })
                        .collect::<HashMap<DbIndexNumber, Arc<BoundColumnFamily>>>(),
                    assets_by_utxo_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(ASSET_BY_ASSET_PK_CF).unwrap(),
                        )
                    },
                    asset_id_by_asset_birth_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(ASSET_ID_BY_ASSET_BIRTH_PK_CF).unwrap(),
                        )
                    },
                    asset_birth_pk_by_asset_id_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(ASSET_BIRTH_PK_BY_ASSET_ID_CF).unwrap(),
                        )
                    },
                    asset_birth_pk_with_asset_pk_cf: unsafe {
                        transmute::<_, Arc<BoundColumnFamily<'static>>>(
                            db.cf_handle(ASSET_BIRTH_PK_WITH_ASSET_PK_CF).unwrap(),
                        )
                    },
                },
            }),
        }
    }
}
