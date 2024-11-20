use byteorder::{BigEndian, ByteOrder};
use model::{BlockHeight, TxHash, TxIndex, TxPk};

use crate::codec::{EncodeDecode, StreamingContext};
pub type TxPkBytes = [u8; 6];

impl EncodeDecode for TxPk {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        self.block_height.encode_internal(buffer, context);
        self.tx_index.encode_internal(buffer, context);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let block_height = BlockHeight::decode_internal(bytes, context);
        let tx_index = TxIndex::decode_internal(bytes, context);

        TxPk {
            block_height,
            tx_index,
        }
    }

    fn size() -> usize {
        BlockHeight::size() + TxIndex::size()
    }
}

impl EncodeDecode for TxIndex {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        let slice = context.next_slice_mut(buffer, Self::size());
        BigEndian::write_u16(slice, self.0);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let slice = context.next_slice(bytes, Self::size());
        TxIndex(BigEndian::read_u16(slice))
    }

    fn size() -> usize {
        2
    }
}

impl EncodeDecode for TxHash {
    fn encode_internal(&self, buffer: &mut [u8], context: &mut StreamingContext) {
        let slice = context.next_slice_mut(buffer, Self::size());
        slice.copy_from_slice(&self.0);
    }

    fn decode_internal(bytes: &[u8], context: &mut StreamingContext) -> Self {
        let slice = context.next_slice(bytes, Self::size());
        let mut hash = [0u8; 32];
        hash.copy_from_slice(slice);
        TxHash(hash)
    }

    fn size() -> usize {
        32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_index_roundtrip() {
        let original = TxIndex(12345);
        let encoded = original.encode();
        let decoded = TxIndex::decode(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_tx_hash_roundtrip() {
        let original = TxHash([3u8; 32]);
        let encoded = original.encode();
        let decoded = TxHash::decode(&encoded);
        assert_eq!(original, decoded);
    }
    #[test]
    fn test_tx_pk_roundtrip() {
        let original = TxPk {
            block_height: BlockHeight(42),
            tx_index: TxIndex(12345),
        };
        let encoded = original.encode();
        let decoded = TxPk::decode(&encoded);
        assert_eq!(original, decoded);
    }
}
