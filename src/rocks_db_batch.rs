use std::sync::Arc;

use rocksdb::BoundColumnFamily;

// Define a trait for blockchain-specific fields
pub trait CustomFamilies<'db> {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'db>>>;
}

pub struct SharedFamilies<'db> {
    pub meta_cf: Arc<BoundColumnFamily<'db>>,
    pub block_hash_by_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub block_pk_by_hash_cf: Arc<BoundColumnFamily<'db>>,
    pub tx_hash_by_pk_cf: Arc<BoundColumnFamily<'db>>,
    pub tx_pk_by_hash_cf: Arc<BoundColumnFamily<'db>>,
}

impl<'db> SharedFamilies<'db> {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'db>>> {
        let mut all = vec![];
        all.push(Arc::clone(&self.meta_cf));
        all.push(Arc::clone(&self.block_hash_by_pk_cf));
        all.push(Arc::clone(&self.block_pk_by_hash_cf));
        all.push(Arc::clone(&self.tx_hash_by_pk_cf));
        all.push(Arc::clone(&self.tx_pk_by_hash_cf));
        all
    }
}

pub struct Families<'db, CF>
where
    CF: CustomFamilies<'db>,
{
    pub shared: SharedFamilies<'db>,
    pub custom: CF,
}

impl<'db, CF> Families<'db, CF>
where
    CF: CustomFamilies<'db>,
{
    pub fn get_all_families(&self) -> Vec<Arc<BoundColumnFamily<'db>>> {
        let mut shared = self.shared.get_all();
        let mut custom = self.custom.get_all();
        let mut all = vec![];
        all.append(&mut shared);
        all.append(&mut custom);
        all
    }
}
