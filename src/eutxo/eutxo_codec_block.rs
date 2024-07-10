use byteorder::{BigEndian, ByteOrder};

use crate::api::{BlockHash, BlockHeight};

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

pub fn vector_to_block_height(block_height_bytes: &Vec<u8>) -> BlockHeight {
    BigEndian::read_u32(&block_height_bytes[0..4])
}

pub fn vector_to_block_hash(block_hash_bytes: &Vec<u8>) -> BlockHash {
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&block_hash_bytes);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_height_to_bytesround_trip() {
        let block_height = 123456;
        let encoded = block_height_to_bytes(&block_height);
        let decoded = bytes_to_block_height(encoded);
        assert_eq!(block_height, decoded);
    }

    #[test]
    fn vector_to_block_height_round_trip() {
        let block_height = 654321;
        let encoded = block_height_to_bytes(&block_height).to_vec();
        let decoded = vector_to_block_height(&encoded);
        assert_eq!(block_height, decoded);
    }

    #[test]
    fn vector_to_block_hash_round_trip() {
        let block_hash: BlockHash = [
            0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ];
        let encoded = block_hash.to_vec();
        let decoded = vector_to_block_hash(&encoded);
        assert_eq!(block_hash, decoded);
    }
}
