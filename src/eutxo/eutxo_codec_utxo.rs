use byteorder::{BigEndian, ByteOrder};

use crate::{
    codec_tx::TxPkBytes,
    model::{BlockHeight, TxIndex},
};

use super::eutxo_model::{UtxoIndex, UtxoValue};

type EutxoPkBytes = [u8; 8];

#[derive(Debug, PartialEq, Clone)]
struct EutxoPk {
    pub block_height: BlockHeight,
    pub tx_index: TxIndex,
    pub utxo_index: UtxoIndex,
}

pub fn utxo_value_to_bytes(utxo_value: &UtxoValue) -> [u8; std::mem::size_of::<UtxoValue>()] {
    let mut bytes = [0u8; 8];
    BigEndian::write_u64(&mut bytes, utxo_value.0);
    bytes
}

pub fn bytes_to_utxo_value(utxo_value_bytes: &[u8]) -> UtxoValue {
    assert_eq!(
        utxo_value_bytes.len(),
        8,
        "utxo value slice must be 8 bytes long"
    );
    BigEndian::read_u64(&utxo_value_bytes).into()
}

pub fn pk_bytes(block_height: &BlockHeight, tx_index: &TxIndex, box_index: &u16) -> EutxoPkBytes {
    let mut bytes: EutxoPkBytes = [0u8; 8];
    BigEndian::write_u32(&mut bytes[0..4], block_height.0);
    BigEndian::write_u16(&mut bytes[4..6], tx_index.0);
    BigEndian::write_u16(&mut bytes[6..8], *box_index);
    bytes
}

pub fn utxo_pk_bytes_from(tx_pk_bytes: Vec<u8>, utxo_index: &UtxoIndex) -> EutxoPkBytes {
    let mut bytes: EutxoPkBytes = [0u8; 8];
    bytes[0..6].copy_from_slice(&tx_pk_bytes);
    BigEndian::write_u16(&mut bytes[6..8], utxo_index.0);
    bytes
}

pub fn utxo_index_from_pk_bytes(utxo_pk_bytes: &[u8]) -> UtxoIndex {
    assert_eq!(utxo_pk_bytes.len(), 8, "utxo pk slice must be 8 bytes long");
    BigEndian::read_u16(&utxo_pk_bytes[6..8]).into()
}

pub fn tx_pk_from_utxo_pk(utxo_pk_bytes: &[u8]) -> TxPkBytes {
    assert_eq!(utxo_pk_bytes.len(), 8, "utxo pk slice must be 8 bytes long");
    let mut bytes: TxPkBytes = [0u8; 6];
    bytes[0..6].copy_from_slice(utxo_pk_bytes);
    bytes
}

impl From<&EutxoPk> for EutxoPkBytes {
    fn from(utxo_id: &EutxoPk) -> EutxoPkBytes {
        pk_bytes(
            &utxo_id.block_height,
            &utxo_id.tx_index,
            &utxo_id.utxo_index.0,
        )
    }
}

impl From<EutxoPkBytes> for EutxoPk {
    fn from(bytes: EutxoPkBytes) -> EutxoPk {
        let block_height: BlockHeight = BigEndian::read_u32(&bytes[0..4]).into();
        let tx_index = BigEndian::read_u16(&bytes[4..6]).into();
        let utxo_index: UtxoIndex = BigEndian::read_u16(&bytes[6..8]).into();
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
            block_height: 123456.into(),
            tx_index: 7890.into(),
            utxo_index: 1234.into(),
        };
        let encoded: EutxoPkBytes = (&utxo_id).into();
        let decoded: EutxoPk = encoded.into();
        assert_eq!(utxo_id, decoded);
    }
    #[test]
    fn test_round_trip_utxo_value_conversion() {
        let value: u64 = 12345678901234567890;
        let encoded = utxo_value_to_bytes(&value.into());
        let decoded = BigEndian::read_u64(&encoded);
        assert_eq!(value, decoded);
    }
}
