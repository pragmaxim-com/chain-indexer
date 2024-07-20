use byteorder::{BigEndian, ByteOrder};

use crate::{
    codec_tx::TxPkBytes,
    model::{BlockHeight, DbIndexCfIndex, TxIndex},
};

use super::eutxo_model::{UtxoIndex, UtxoValue};

pub type UtxoPkBytes = [u8; 8];
pub type UtxoValueWithIndexes = Vec<u8>;
pub type UtxoBirthPkBytes = UtxoPkBytes;
type UtxoBirthPkWithUtxoPkBytes = [u8; 16];

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

pub fn concat_utxo_birth_pk_with_utxo_pk(
    utxo_birth_pk_bytes: &[u8],
    utxo_pk_bytes: &UtxoPkBytes,
) -> UtxoBirthPkWithUtxoPkBytes {
    let mut combined_bytes = [0u8; 16];

    combined_bytes[0..8].copy_from_slice(utxo_birth_pk_bytes);
    combined_bytes[8..16].copy_from_slice(utxo_pk_bytes);

    combined_bytes
}

pub fn bytes_to_utxo(bytes: &[u8]) -> (UtxoValue, Vec<(DbIndexCfIndex, UtxoPkBytes)>) {
    let utxo_value = UtxoValue(BigEndian::read_u64(&bytes[0..8]));
    let num_pairs = (bytes.len() - 8) / 9;
    let mut utxo_birth_pk_by_cf_index = Vec::with_capacity(num_pairs);
    let mut index = 8;
    while index < bytes.len() {
        if index + 9 <= bytes.len() {
            let cf_index_id = bytes[index];
            index += 1;
            let utxo_birth_pk = &bytes[index..index + 8];
            index += 8;
            let utxo_birth_pk_bytes: UtxoPkBytes = <UtxoPkBytes>::try_from(utxo_birth_pk)
                .expect("UtxoBirthPk should have exactly 8 bytes");
            utxo_birth_pk_by_cf_index.push((cf_index_id, utxo_birth_pk_bytes));
        } else {
            break;
        }
    }
    (utxo_value, utxo_birth_pk_by_cf_index)
}

pub fn utxo_to_bytes(
    utxo_value: UtxoValue,
    utxo_birth_pk_by_cf_index: Vec<(DbIndexCfIndex, UtxoPkBytes)>,
) -> Vec<u8> {
    let mut utxo_value_with_indexes = vec![0u8; 8 + utxo_birth_pk_by_cf_index.len() * 9];
    BigEndian::write_u64(&mut utxo_value_with_indexes[0..8], utxo_value.0);

    let mut index = 8;
    for (db_index_id, utxo_birth_pk) in utxo_birth_pk_by_cf_index {
        utxo_value_with_indexes[index] = db_index_id;
        index += 1;
        utxo_value_with_indexes[index..index + 8].copy_from_slice(&utxo_birth_pk);
        index += 8;
    }

    utxo_value_with_indexes
}

pub fn pk_bytes(block_height: &BlockHeight, tx_index: &TxIndex, box_index: &u16) -> UtxoPkBytes {
    let mut bytes: UtxoPkBytes = [0u8; 8];
    BigEndian::write_u32(&mut bytes[0..4], block_height.0);
    BigEndian::write_u16(&mut bytes[4..6], tx_index.0);
    BigEndian::write_u16(&mut bytes[6..8], *box_index);
    bytes
}

pub fn utxo_pk_bytes_from(tx_pk_bytes: Vec<u8>, utxo_index: &UtxoIndex) -> UtxoPkBytes {
    let mut bytes: UtxoPkBytes = [0u8; 8];
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

impl From<&EutxoPk> for UtxoPkBytes {
    fn from(utxo_id: &EutxoPk) -> UtxoPkBytes {
        pk_bytes(
            &utxo_id.block_height,
            &utxo_id.tx_index,
            &utxo_id.utxo_index.0,
        )
    }
}

impl From<UtxoPkBytes> for EutxoPk {
    fn from(bytes: UtxoPkBytes) -> EutxoPk {
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
        let encoded: UtxoPkBytes = (&utxo_id).into();
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

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_roundtrip(utxo_value in any::<u64>(), pairs in prop::collection::vec((any::<u8>(), any::<[u8; 8]>()), 0..100)) {
            let utxo_value = UtxoValue(utxo_value);
            let bytes = utxo_to_bytes(utxo_value, pairs.clone());
            let (decoded_utxo_value, decoded_pairs) = bytes_to_utxo(&bytes);
            assert_eq!(utxo_value.0, decoded_utxo_value.0);
            assert_eq!(pairs, decoded_pairs);
        }
    }
}
