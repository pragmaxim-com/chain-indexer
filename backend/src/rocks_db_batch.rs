use std::sync::Arc;

use rocksdb::BoundColumnFamily;

// Define a trait for blockchain-specific fields
pub trait CustomFamilies {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'static>>>;
}

pub struct SharedFamilies {
    pub meta_cf: Arc<BoundColumnFamily<'static>>,
    pub block_hash_by_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub block_header_by_hash_cf: Arc<BoundColumnFamily<'static>>,
    pub tx_hash_by_pk_cf: Arc<BoundColumnFamily<'static>>,
    pub tx_pk_by_hash_cf: Arc<BoundColumnFamily<'static>>,
}

impl SharedFamilies {
    fn get_all(&self) -> Vec<Arc<BoundColumnFamily<'static>>> {
        let all = vec![
            Arc::clone(&self.meta_cf),
            Arc::clone(&self.block_hash_by_pk_cf),
            Arc::clone(&self.block_header_by_hash_cf),
            Arc::clone(&self.tx_hash_by_pk_cf),
            Arc::clone(&self.tx_pk_by_hash_cf),
        ];
        all
    }
}

pub struct Families<CF>
where
    CF: CustomFamilies,
{
    pub shared: SharedFamilies,
    pub custom: CF,
}

impl<CF> Families<CF>
where
    CF: CustomFamilies,
{
    pub fn get_all_families(&self) -> Vec<Arc<BoundColumnFamily<'static>>> {
        let mut shared = self.shared.get_all();
        let mut custom = self.custom.get_all();
        let mut all = vec![];
        all.append(&mut shared);
        all.append(&mut custom);
        all
    }
}
