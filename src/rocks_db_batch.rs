use rocksdb::ColumnFamily;

// Define a trait for blockchain-specific fields
pub trait CustomFamilies<'db> {
    fn get_all(&self) -> Vec<&'db ColumnFamily>;
}

pub struct SharedFamilies<'db> {
    pub(crate) meta_cf: &'db ColumnFamily,
    pub(crate) block_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) block_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) tx_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) tx_pk_by_hash_cf: &'db ColumnFamily,
}

impl<'db> SharedFamilies<'db> {
    fn get_all(&self) -> Vec<&'db ColumnFamily> {
        let mut all = vec![];
        all.push(self.meta_cf);
        all.push(self.block_hash_by_pk_cf);
        all.push(self.block_pk_by_hash_cf);
        all.push(self.tx_hash_by_pk_cf);
        all.push(self.tx_pk_by_hash_cf);
        all
    }
}

pub struct Families<'db, CF>
where
    CF: CustomFamilies<'db>,
{
    pub(crate) shared: SharedFamilies<'db>,
    pub(crate) custom: CF,
}

impl<'db, CF> Families<'db, CF>
where
    CF: CustomFamilies<'db>,
{
    pub fn get_all_families(&self) -> Vec<&'db ColumnFamily> {
        let mut shared = self.shared.get_all();
        let mut custom = self.custom.get_all();
        let mut all = vec![];
        all.append(&mut shared);
        all.append(&mut custom);
        all
    }
}
