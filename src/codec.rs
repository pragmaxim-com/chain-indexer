use byteorder::{BigEndian, ByteOrder};

type UtxoIdBytes = [u8; 8];

#[derive(Debug, PartialEq, Clone)]
struct CiUtxoId {
    pub block_height: u32,
    pub tx_index: u16,
    pub utxo_index: u16,
}

// Implementing From trait for CiUtxoId to UtxoIdBytes conversion
impl From<CiUtxoId> for UtxoIdBytes {
    fn from(utxo_id: CiUtxoId) -> UtxoIdBytes {
        let mut bytes: UtxoIdBytes = [0u8; 8];
        BigEndian::write_u32(&mut bytes[0..4], utxo_id.block_height);
        BigEndian::write_u16(&mut bytes[4..6], utxo_id.tx_index);
        BigEndian::write_u16(&mut bytes[6..8], utxo_id.utxo_index);
        bytes
    }
}

// Implementing From trait for UtxoIdBytes to CiUtxoId conversion
impl From<UtxoIdBytes> for CiUtxoId {
    fn from(bytes: UtxoIdBytes) -> CiUtxoId {
        let block_height = BigEndian::read_u32(&bytes[0..4]);
        let tx_index = BigEndian::read_u16(&bytes[4..6]);
        let utxo_index = BigEndian::read_u16(&bytes[6..8]);
        CiUtxoId {
            block_height,
            tx_index,
            utxo_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_utxo_id() {
        let utxo_id = CiUtxoId {
            block_height: 123456,
            tx_index: 7890,
            utxo_index: 1234,
        };
        let expected_bytes: UtxoIdBytes = [0, 1, 226, 64, 30, 222, 4, 210];
        let encoded: UtxoIdBytes = utxo_id.into();
        assert_eq!(encoded, expected_bytes);
    }

    #[test]
    fn test_decode_utxo_id() {
        let bytes: UtxoIdBytes = [0, 1, 226, 64, 30, 222, 4, 210];
        let expected_utxo_id = CiUtxoId {
            block_height: 123456,
            tx_index: 7890,
            utxo_index: 1234,
        };
        let decoded: CiUtxoId = bytes.into();
        assert_eq!(decoded, expected_utxo_id);
    }

    #[test]
    fn test_round_trip_conversion() {
        let utxo_id = CiUtxoId {
            block_height: 123456,
            tx_index: 7890,
            utxo_index: 1234,
        };
        let encoded: UtxoIdBytes = utxo_id.clone().into();
        let decoded: CiUtxoId = encoded.into();
        assert_eq!(utxo_id, decoded);
    }
}
