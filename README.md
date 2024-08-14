## DB schema

Chain indexer is a universal blockchain indexing tool on top of RocksDB that generates one-to-many and one-to-one indexes to be able to answer all sorts of explorer queries.
Indexer uses block/tx/box indexes over hashes which allows for much better space efficiency and for ~ 3 000 - 6 000 txs/s speed, depending on ammount of outputs/assets and WAL being on/off. Chain tip is "eventually consistent" due to using indexes over hashes, ie. forks get settled eventually.

Currently Bitcoin, Cardano and Ergo are supported.

### Data model

```
PK           = unique pointer to an object
BirthPK      = unique pointer to an object of creation
Hash         = Hash of an object
Index        = Secondary Index
Asset Action = Mint, Transfer, Burn
```
```
HeightPk     = block_height
TxPk         = block_height|tx_index
InputPk      = block_height|tx_index|input_index
UtxoPk       = block_height|tx_index|utxo_index
UtxoBirthPk  = block_height|tx_index|utxo_index
AssetPk      = block_height|tx_index|utxo_index|asset_index
AssetBirthPk = block_height|tx_index|utxo_index|asset_index
```

**Meta column family** keeps track of last block header we indexed. Indexing is completely idempotent and blocks are persited atomicly (in a db transaction).

**UtxoIndexes** and **AssetIndex** are seconary indexes that keep entity (`asset/address/script_hash/etc...`) under small-size `UtxoBirthPk/AssetBirthPk`
and references/relations to all further occurences to them.

> Note, that there can be 0-x of either one-to-many or one-to-one UtxoIndexes, while AssetIndex is curently only 1 one-to-many for all assets together

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

Secondary indexes like (`script_hash/address`) are stored as sequence of pointers to a utxo where it was first born, prefixed with a column family pointer.
`UtxoPk_by_InputPk` is used to tell whether box is spent or not.

```
UtxoValueAndUtxoBirthPks_by_UtxoPk:
    utxo_pk -> utxo_value|[utxo_index_cf:utxo_birth_pk,utxo_index_cf:utxo_birth_pk]

Spent_UtxoPk_by_InputPk:
    input_pk -> utxo_pk

Spent_InputPk_by_UtxoPk:
    utxo_pk -> input_pk
```

### Utxo indexes (one-to-many)

We keep secondary indexes (`script_hash/address`) under small-size `utxo_birth_pk` identifiers which is a unique pointer of their creation.
Then we keep relations to all following boxes where given indexed entity occurred. Following table shows 2 example secondary indexes : `script_hash` & `address`.

> Note that one-to-one indexes are the same, just without `relations`.

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

Assets are for now just basic with single one-to-many secondary index 

```
AssetValueAndBirthPk_by_UtxoPk:
    asset_pk -> asset_value|asset_birth_pk|action
```

### Asset index

System of secondary indexes is applied the same as for other parts of a Box.

```
AssetId_by_AssetBirthPk
    asset_birth_pk -> asset_id

AssetBirthPk_by_AssetId
    asset_id -> asset_birth_pk

AssetBirthPk_with_AssetPk_relations:
    asset_birth_pk|asset_pk
```