use byteorder::{BigEndian, ByteOrder};
use model::{BlockHash, BlockHeader, BlockHeight, BlockTimestamp};

use crate::codec::{EncodeDecode, NestedEncodeDecode, StreamingContext};

// Implementation for simple types
impl EncodeDecode for BlockHeight {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        let slice = context.next_slice_mut(buffer, Self::size());
        BigEndian::write_u32(slice, self.0);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let slice = context.next_slice(bytes, Self::size());
        BlockHeight(BigEndian::read_u32(slice))
    }

    fn size() -> usize {
        4
    }
}

impl EncodeDecode for BlockTimestamp {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        let slice = context.next_slice_mut(buffer, Self::size());
        BigEndian::write_u32(slice, self.0);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let slice = context.next_slice(bytes, Self::size());
        BlockTimestamp(BigEndian::read_u32(slice))
    }

    fn size() -> usize {
        4
    }
}

impl EncodeDecode for BlockHash {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        let slice = context.next_slice_mut(buffer, Self::size());
        slice.copy_from_slice(&self.0);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let slice = context.next_slice(bytes, Self::size());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(slice);
        BlockHash(hash)
    }

    fn size() -> usize {
        32
    }
}

// Implementation for nested type BlockHeader
impl EncodeDecode for BlockHeader {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        self.height.encode_internal(buffer, context);
        self.timestamp.encode_internal(buffer, context);
        self.hash.encode_internal(buffer, context);
        self.prev_hash.encode_internal(buffer, context);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let height = BlockHeight::decode_internal(bytes, context);
        let timestamp = BlockTimestamp::decode_internal(bytes, context);
        let hash = BlockHash::decode_internal(bytes, context);
        let prev_hash = BlockHash::decode_internal(bytes, context);

        BlockHeader {
            height,
            timestamp,
            hash,
            prev_hash,
        }
    }

    fn size() -> usize {
        BlockHeight::size() + BlockTimestamp::size() + 2 * BlockHash::size()
    }
}

impl NestedEncodeDecode for BlockHeader {}

#[cfg(test)]
mod tests {
    use super::*;
    use model::{BlockHash, BlockHeader, BlockHeight, BlockTimestamp};

    #[test]
    fn test_block_height_roundtrip() {
        let original = BlockHeight(42);
        let encoded = original.encode();
        let decoded = BlockHeight::decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_block_timestamp_roundtrip() {
        let original = BlockTimestamp(1629394872);
        let encoded = original.encode();
        let decoded = BlockTimestamp::decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_block_hash_roundtrip() {
        let original = BlockHash([1u8; 32]);
        let encoded = original.encode();
        let decoded = BlockHash::decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_block_header_roundtrip() {
        let original = BlockHeader {
            height: BlockHeight(42),
            timestamp: BlockTimestamp(1629394872),
            hash: BlockHash([1u8; 32]),
            prev_hash: BlockHash([2u8; 32]),
        };
        let encoded = original.encode();
        let decoded = BlockHeader::decode(&encoded);
        assert_eq!(original, decoded);
    }
}
