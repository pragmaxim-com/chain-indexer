use byteorder::{BigEndian, ByteOrder};

use crate::api::BlockHeight;

pub fn block_height_to_bytes(
    block_height: &BlockHeight,
) -> [u8; std::mem::size_of::<BlockHeight>()] {
    let mut bytes = [0u8; 4];
    BigEndian::write_u32(&mut bytes, *block_height);
    bytes
}

pub fn bytes_to_block_height(block_height_bytes: [u8; 4]) -> BlockHeight {
    BigEndian::read_u32(&block_height_bytes[0..4])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_conversion() {
        let block_height = 123456;
        let encoded = block_height_to_bytes(&block_height);
        let decoded = bytes_to_block_height(encoded);
        assert_eq!(block_height, decoded);
    }
}
