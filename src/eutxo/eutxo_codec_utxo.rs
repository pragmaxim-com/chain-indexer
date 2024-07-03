use byteorder::{BigEndian, ByteOrder};

use crate::api::{BlockHeight, TxIndex};

use super::eutxo_api::{UtxoIndex, UtxoValue};

type EutxoPkBytes = [u8; 8];

#[derive(Debug, PartialEq, Clone)]
struct EutxoPk {
    pub block_height: BlockHeight,
    pub tx_index: TxIndex,
    pub utxo_index: UtxoIndex,
}

pub fn utxo_value_to_bytes(utxo_value: &UtxoValue) -> [u8; std::mem::size_of::<UtxoValue>()] {
    let mut bytes = [0u8; 8];
    BigEndian::write_u64(&mut bytes, *utxo_value);
    bytes
}

pub fn pk_bytes(block_height: &BlockHeight, tx_index: &TxIndex, box_index: &u16) -> EutxoPkBytes {
    let mut bytes: EutxoPkBytes = [0u8; 8];
    BigEndian::write_u32(&mut bytes[0..4], *block_height);
    BigEndian::write_u16(&mut bytes[4..6], *tx_index);
    BigEndian::write_u16(&mut bytes[6..8], *box_index);
    bytes
}

pub fn utxo_pk_bytes_from(tx_pk_bytes: Vec<u8>, utxo_index: UtxoIndex) -> EutxoPkBytes {
    let mut bytes: EutxoPkBytes = [0u8; 8];
    bytes[0..6].copy_from_slice(&tx_pk_bytes);
    BigEndian::write_u16(&mut bytes[6..8], utxo_index);
    bytes
}

impl From<&EutxoPk> for EutxoPkBytes {
    fn from(utxo_id: &EutxoPk) -> EutxoPkBytes {
        pk_bytes(
            &utxo_id.block_height,
            &utxo_id.tx_index,
            &utxo_id.utxo_index,
        )
    }
}

impl From<EutxoPkBytes> for EutxoPk {
    fn from(bytes: EutxoPkBytes) -> EutxoPk {
        let block_height = BigEndian::read_u32(&bytes[0..4]);
        let tx_index = BigEndian::read_u16(&bytes[4..6]);
        let utxo_index = BigEndian::read_u16(&bytes[6..8]);
        EutxoPk {
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
    fn test_round_trip_conversion() {
        let utxo_id = EutxoPk {
            block_height: 123456,
            tx_index: 7890,
            utxo_index: 1234,
        };
        let encoded: EutxoPkBytes = (&utxo_id).into();
        let decoded: EutxoPk = encoded.into();
        assert_eq!(utxo_id, decoded);
    }
    #[test]
    fn test_round_trip_utxo_value_conversion() {
        let value: u64 = 12345678901234567890;
        let encoded = utxo_value_to_bytes(&value);
        let decoded = BigEndian::read_u64(&encoded);
        assert_eq!(value, decoded);
    }
}
