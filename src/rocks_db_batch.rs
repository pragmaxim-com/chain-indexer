use rocksdb::ColumnFamily;

// Define a trait for blockchain-specific fields
pub trait CustomFamilies<'db> {}

pub struct SharedFamilies<'db> {
    pub(crate) meta_cf: &'db ColumnFamily,
    pub(crate) block_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) block_pk_by_hash_cf: &'db ColumnFamily,
    pub(crate) tx_hash_by_pk_cf: &'db ColumnFamily,
    pub(crate) tx_pk_by_hash_cf: &'db ColumnFamily,
}

pub struct Families<'db, CF>
where
    CF: CustomFamilies<'db>,
{
    pub(crate) shared: SharedFamilies<'db>,
    pub(crate) custom: CF,
}
