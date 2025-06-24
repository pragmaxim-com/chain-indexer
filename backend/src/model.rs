use chrono::DateTime;
use core::fmt;
use derive_more::{AsRef, From, Into};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

pub type TxCount = usize;
pub type BoxWeight = usize;
pub type BatchWeight = usize;

#[derive(Clone, Copy, Debug, IntoPrimitive, PartialEq, TryFromPrimitive, )]
#[repr(u8)]
pub enum AssetType {
    Mint = 0,
    Transfer = 1,
    Burn = 2,
}
