use byteorder::{BigEndian, ByteOrder};

use crate::model::{BlockHash, BlockHeader, BlockHeight, BlockTimestamp};

type BlockHeightBytes = [u8; 4];
type BlockHeaderBytes = [u8; 72];

pub fn block_header_to_bytes(block_header: &BlockHeader) -> BlockHeaderBytes {
    let mut bytes = [0u8; 72];
    BigEndian::write_u32(&mut bytes[0..4], block_header.height.0);
    BigEndian::write_u32(&mut bytes[4..8], block_header.timestamp.0);
    bytes[8..40].copy_from_slice(&block_header.hash.0);
    bytes[40..72].copy_from_slice(&block_header.prev_hash.0);
    bytes
}

pub fn bytes_to_block_header(header_bytes: &[u8]) -> BlockHeader {
    assert_eq!(header_bytes.len(), 72, "header slice must be 40 bytes long");

    let height: BlockHeight = BigEndian::read_u32(&header_bytes[0..4]).into();
    let timestamp: BlockTimestamp = BigEndian::read_u32(&header_bytes[4..8]).into();

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&header_bytes[8..40]);

    let mut prev_hash = [0u8; 32];
    prev_hash.copy_from_slice(&header_bytes[40..72]);

    BlockHeader {
        height,
        timestamp,
        hash: hash.into(),
        prev_hash: prev_hash.into(),
    }
}

pub fn block_height_to_bytes(block_height: &BlockHeight) -> BlockHeightBytes {
    let mut bytes = [0u8; 4];
    BigEndian::write_u32(&mut bytes, block_height.0);
    bytes
}

pub fn bytes_to_block_height(block_height_bytes: &[u8]) -> BlockHeight {
    assert_eq!(
        block_height_bytes.len(),
        4,
        "block height must be 4 bytes long"
    );
    BigEndian::read_u32(&block_height_bytes[0..4]).into()
}

pub fn bytes_to_block_hash(block_hash_bytes: &[u8]) -> BlockHash {
    assert_eq!(
        block_hash_bytes.len(),
        32,
        "Block hash bytes must be 32 bytes long"
    );
    let mut hash: [u8; 32] = [0u8; 32];
    hash.copy_from_slice(&block_hash_bytes);
    hash.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_height_to_bytesround_trip() {
        let block_height = 123456;
        let encoded = block_height_to_bytes(&block_height.into());
        let decoded = bytes_to_block_height(&encoded);
        assert_eq!(block_height, decoded.0);
    }

    #[test]
    fn vector_to_block_height_round_trip() {
        let block_height = 654321;
        let encoded = block_height_to_bytes(&block_height.into()).to_vec();
        let decoded = bytes_to_block_height(&encoded);
        assert_eq!(block_height, decoded.0);
    }

    #[test]
    fn vector_to_block_hash_round_trip() {
        let block_hash: BlockHash = [
            0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]
        .into();
        let encoded: Vec<u8> = block_hash.0.to_vec();
        let decoded = bytes_to_block_hash(&encoded);
        assert_eq!(block_hash, decoded);
    }

    #[test]
    fn test_block_header_roundtrip() {
        let original_block_header = BlockHeader {
            height: 42.into(),
            timestamp: 1625156400.into(),
            hash: [0xaa; 32].into(),
            prev_hash: [0xbb; 32].into(),
        };

        // Convert to bytes
        let bytes = block_header_to_bytes(&original_block_header);

        // Convert back to BlockHeader
        let decoded_block_header = bytes_to_block_header(&bytes);

        assert_eq!(decoded_block_header, original_block_header);
    }
}
