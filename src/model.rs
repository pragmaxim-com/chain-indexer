use chrono::DateTime;
use core::fmt;
use derive_more::{AsRef, Display, From, Into};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeader {
    pub height: BlockHeight,
    pub timestamp: BlockTimestamp,
    pub hash: BlockHash,
    pub prev_hash: BlockHash,
}
impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} @ {} : {} <- {}",
            self.height, self.timestamp, self.hash, self.prev_hash,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From)]
pub struct BlockTimestamp(pub u32);
impl fmt::Display for BlockTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let datetime = DateTime::from_timestamp(self.0 as i64, 0).unwrap();
        let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        write!(f, "{}", readable_date)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct BlockHeight(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct BlockHash(pub [u8; 32]);
impl AsRef<[u8]> for BlockHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

pub type TxCount = usize;

#[derive(Debug, Clone, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct TxIndex(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct TxHash(pub [u8; 32]);
impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
impl AsRef<[u8]> for TxHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl TryFrom<Box<[u8]>> for TxHash {
    type Error = &'static str;

    fn try_from(boxed_slice: Box<[u8]>) -> Result<Self, Self::Error> {
        if boxed_slice.len() == 32 {
            let boxed_array: Box<[u8; 32]> = match boxed_slice.try_into() {
                Ok(arr) => arr,
                Err(_) => return Err("Failed to convert Box<[u8]> into Box<[u8; 32]>"),
            };
            Ok(TxHash(*boxed_array))
        } else {
            Err("Box<[u8]> does not have exactly 32 bytes")
        }
    }
}

pub type AssetId = Vec<u8>;
pub type AssetValue = u64;

pub type DbIndexCfIndex = u8;
pub type DbIndexAgidWithUtxoPkCf = String;
pub type DbIndexByAgidCf = String;
pub type DbAgidByIndexCf = String;
pub type DbIndexAgid = u32;
pub type DbIndexValue = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block<T: Clone> {
    pub header: BlockHeader,
    pub txs: Vec<T>,
}

impl<T: Clone> Block<T> {
    pub fn new(header: BlockHeader, txs: Vec<T>) -> Self {
        Self { header, txs }
    }
}

pub trait Transaction {
    fn is_coinbase(&self) -> bool;
    fn hash(&self) -> &TxHash;
    fn index(&self) -> &TxIndex;
}
