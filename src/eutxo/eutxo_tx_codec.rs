use byteorder::{BigEndian, ByteOrder};

type TxPkBytes = [u8; 6];

#[derive(Debug, PartialEq, Clone)]
struct TxPk {
    pub block_height: u32,
    pub tx_index: u16,
}

pub fn tx_pk_bytes(block_height: &u32, tx_index: &u16) -> TxPkBytes {
    let mut bytes: TxPkBytes = [0u8; 6];
    BigEndian::write_u32(&mut bytes[0..4], *block_height);
    BigEndian::write_u16(&mut bytes[4..6], *tx_index);
    bytes
}

// Implementing From trait for Tx to TxBytes conversion
impl From<TxPk> for TxPkBytes {
    fn from(tx: TxPk) -> TxPkBytes {
        tx_pk_bytes(&tx.block_height, &tx.tx_index)
    }
}

// Implementing From trait for TxBytes to Tx conversion
impl From<TxPkBytes> for TxPk {
    fn from(bytes: TxPkBytes) -> TxPk {
        let block_height = BigEndian::read_u32(&bytes[0..4]);
        let tx_index = BigEndian::read_u16(&bytes[4..6]);
        TxPk {
            block_height,
            tx_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_conversion() {
        let tx = TxPk {
            block_height: 123456,
            tx_index: 7890,
        };
        let encoded: TxPkBytes = tx.clone().into();
        let decoded: TxPk = encoded.into();
        assert_eq!(tx, decoded);
    }
}
