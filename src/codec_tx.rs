use byteorder::{BigEndian, ByteOrder};

use crate::model::{BlockHeight, TxIndex};

pub type TxPkBytes = [u8; 6];

pub fn tx_pk_bytes(block_height: &BlockHeight, tx_index: &TxIndex) -> TxPkBytes {
    let mut bytes: TxPkBytes = [0u8; 6];
    BigEndian::write_u32(&mut bytes[0..4], block_height.0);
    BigEndian::write_u16(&mut bytes[4..6], tx_index.0);
    bytes
}

pub fn pk_bytes_to_tx(bytes: TxPkBytes) -> (BlockHeight, TxIndex) {
    let block_height: BlockHeight = BigEndian::read_u32(&bytes[0..4]).into();
    let tx_index: TxIndex = BigEndian::read_u16(&bytes[4..6]).into();
    (block_height, tx_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_conversion() {
        let block_height: BlockHeight = 123456.into();
        let tx_index: TxIndex = 7890.into();
        let encoded: TxPkBytes = tx_pk_bytes(&block_height, &tx_index);
        let (h, ti) = pk_bytes_to_tx(encoded);
        assert_eq!(block_height, h);
        assert_eq!(tx_index, ti);
    }
}
