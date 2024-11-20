pub mod eutxo_model;

use chrono::DateTime;
use core::fmt;
use derive_more::{AsRef, Display, From, Into};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type TxCount = usize;
pub type BoxWeight = usize;
pub type BlockWeight = usize;
pub type BatchWeight = usize;
use serde::de::Error as DeError;

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct O2mIndexValue(pub Vec<u8>);
impl fmt::Display for O2mIndexValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl Serialize for O2mIndexValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = hex::encode(&self.0);
        serializer.serialize_str(&hex_string)
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct O2oIndexValue(pub Vec<u8>);
impl fmt::Display for O2oIndexValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

impl Serialize for O2oIndexValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = hex::encode(&self.0);
        serializer.serialize_str(&hex_string)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct TxIndex(pub u16);

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From)]
pub struct TxPk {
    pub block_height: BlockHeight,
    pub tx_index: TxIndex,
}
impl fmt::Display for TxPk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.block_height, self.tx_index)
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct TxHash(pub [u8; 32]);
impl fmt::Display for TxHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
impl Serialize for TxHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = hex::encode(self.0);
        serializer.serialize_str(&hex_string)
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From)]
pub struct BlockTimestamp(pub u32);
impl fmt::Display for BlockTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let datetime = DateTime::from_timestamp(self.0 as i64, 0).unwrap();
        let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        write!(f, "{}", readable_date)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Display)]
pub struct BlockHeight(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct BlockHash(pub [u8; 32]);
impl AsRef<[u8]> for BlockHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl From<String> for BlockHash {
    fn from(hex_string: String) -> Self {
        let bytes = hex::decode(hex_string).expect("Failed to decode hex string");
        assert!(
            bytes.len() == 32,
            "Hex string must decode to exactly 32 bytes"
        );
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);

        BlockHash(array)
    }
}
impl fmt::Display for BlockHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl Serialize for BlockHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = hex::encode(self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for BlockHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_string = String::deserialize(deserializer)?;
        let bytes = hex::decode(&hex_string).map_err(DeError::custom)?;

        if bytes.len() != 32 {
            return Err(DeError::custom("Invalid length for BlockHash"));
        }

        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(BlockHash(array))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block<T> {
    pub header: BlockHeader,
    pub txs: Vec<T>,
    pub weight: BlockWeight,
}

impl<T> Block<T> {
    pub fn new(header: BlockHeader, txs: Vec<T>, weight: BlockWeight) -> Self {
        Self {
            header,
            txs,
            weight,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, AsRef, Into, From, Hash)]
pub struct AssetId(pub Vec<u8>);
impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

pub type AssetIndex = u8;
pub type AssetValue = u64;
pub type AssetMinted = bool;

#[derive(
    Clone, Serialize, Deserialize, Copy, Debug, IntoPrimitive, PartialEq, TryFromPrimitive,
)]
#[repr(u8)]
pub enum AssetAction {
    Mint = 0,
    Transfer = 1,
    Burn = 2,
}
