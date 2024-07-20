## DB schema

PK           = unique pointer to an object
BirthPK      = unique pointer to an object of creation
Hash         = Hash of an object
Index        = Secondary Index

HeightPk     = block_height
TxPk         = block_height|tx_index
InputPk      = block_height|tx_index|input_index
UtxoPk       = block_height|tx_index|utxo_index
UtxoBirthPk  = block_height|tx_index|utxo_index
AssetPk      = block_height|tx_index|utxo_index|asset_index
AssetBirthPk = block_height|tx_index|utxo_index|asset_index

UtxoIndexes and AssetIndex are seconary index that keeps entity (asset-id/address/script_hash) under small-size UtxoBirthPk/AssetBirthPk
and references/relations to all further occurences to them.

Note, that UtxoIndexes are custom and can be 0-x of them, while AssetIndex is only one, `utxo_index_cf` is encoded as u8
------------------------------------------------------------

### Block

We keep `block_hash` uder small-size unique pointer that we use in the rest of the model to refer to the block.

```
BlockHash_by_HeightPk:
    block_height -> block_hash

HeightPk_by_BlockHash:
    block_hash -> block_height|block_timestamp
```

### Transactions

We keep `tx_hash` under small-size unique pointer that we use in the rest of the model to refer to the Tx.

```
TxHash_by_TxPk:
    tx_pk -> tx_hash

TxPk_by_txHash:
    tx_hash -> tx_pk
```

### Utxo

Secondary indexes like (script_hash / address) are stored as sequence of pointers to a utxo where it was first born, prefixed with a column family pointer.
`UtxoPk_by_InputPk` is used to tell whether box is spent or not.

```
UtxoValueAndUtxoBirthPks_by_UtxoPk:
    utxo_pk -> utxo_value|[utxo_index_cf:utxo_birth_pk,utxo_index_cf:utxo_birth_pk]

UtxoPk_by_InputPk:
    input_pk -> utxo_pk
```

## Utxo indexes

We keep secondary indexes (script_hash / address) under small-size `utxo_birth_pk` identifiers which is a unique pointer of their creation.
Then we keep relations to all following boxes where given indexed entity occurred. Following table shows 2 example secondary indexes script_hash & address.

```
UtxoIndex_by_UtxoBirthPk
    index_by_utxo_birth_pk_address_cf: 
        address_utxo_birth_pk -> address
    index_by_utxo_birth_pk_script_hash_cf: 
        script_hash_utxo_birth_pk -> script_hash

UtxoBirthPk_by_UtxoIndex
    utxo_birth_pk_by_index_address_cf: 
        address -> addres_utxo_birth_pk
    utxo_birth_pk_by_index_script_hash_cf: 
        script_hash -> script_hash_utxo_birth_pk

UtxoBirthPk_with_UtxoPk_relations:
    utxo_birth_pk_by_index_address_cf: 
       address_utxo_birth_pk|utxo_pk
    utxo_birth_pk_by_script_hash_cf: 
       script_hash_utxo_birth_pk|utxo_pk
```

### Assets

To keep data small, we keep assets as an array under utxo_pk.

```
AssetValueAndBirthPk_by_UtxoPk:
    utxo_pk -> [asset_index|asset_value|asset_birth_pk]
```

### Asset index

System of secondary indexes is applied the same as for other parts of a Box.

```
AssetId_by_AssetAgid
    asset_birth_pk -> asset_id

AssetAgid_by_AssetId
    asset_id -> asset_birth_pk

AssetAgid_with_AssetPk_relations:
    asset_birth_pk|asset_pk
```