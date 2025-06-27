use num_enum::{IntoPrimitive, TryFromPrimitive};

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
