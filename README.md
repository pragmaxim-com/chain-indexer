## Chain Indexer

Chain indexer is a universal blockchain indexing tool on top of RocksDB that generates `one-to-many` and `one-to-one` output-based indexes to be able to answer all sorts of explorer queries.

Indexer uses tiny `block_height/tx_index/box_index` pointers over big hashes, ie. not a single hash is duplicated, which allows for much better space efficiency and for ~ `6 000 / 12 000 Inputs+Outputs+Assets per second` throughput with just quad-core and the slowest SSD, depending on `WAL` being on/off. `WAL` disabling is currently useless as [rocksdb flushing does not work](https://github.com/rust-rocksdb/rust-rocksdb/issues/900).

Chain tip is "eventually consistent" due to using pointers over hashes, ie. forks get settled eventually and superseded forks are deleted from DB.

Currently `Bitcoin`, `Cardano` and `Ergo` are supported.

### Installation (Debian/Ubuntu)

```
sudo apt-get install rustup, gcc, g++, libclang-dev, librocksdb-dev
```

### Usage

```
cat bitcoin.conf | grep rpc
rpcthreads=40
rpcworkqueue=512
rpcuser=foo
rpcpassword=bar
rpcallowip=10.0.1.0/24
rpcport=8332
rpcbind=0.0.0.0

export BITCOIN__API_USERNAME="foo"
export BITCOIN__API_PASSWORD="bar"
cargo run -- --blockchain bitcoin

# Cardano node is expected to run locally at port 1337, set socket_path at config/settings.toml
cargo run -- --blockchain cardano

# Ergo node is expected to run locally at port 9053
export ERGO__API_KEY="foo"
cargo run -- --blockchain ergo
```

Querying currently times out during historical indexing. So use it only at the chain tip sync phase 
or when indexing is disabled `indexer.enable = false` and we only run http server to query over existing data :
```
curl -X GET http://127.0.0.1:3031/blocks/123 | jq
curl -X GET http://127.0.0.1:3031/blocks/latest | jq
```

### Data model

```
PK           = unique pointer to an object
BirthPK      = unique pointer to an object of creation
Hash         = Hash of an object
Index        = Secondary Index
Asset Action = Mint / Transfer
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

**one_to_many** seconary indexes keep entity like (`asset/address/script_hash/etc...`) under small-size `UtxoBirthPk/AssetBirthPk`
and references/relations to all further occurences to them.

> Note, that there can be 0-x of either one-to-many or one-to-one UtxoIndexes, while AssetIndex is curently only 1 one-to-many for all assets together

### Block

We keep `block_hash` uder small-size unique pointer that we use in the rest of the model to refer to the block. Possible battle of forks
is ongoing until the longest fork wins and the others are deleted, ie. each new block from a competitive fork always causes deletion of the competitor's fork.

```
BlockHash_by_HeightPk:
    block_height -> block_hash

HeightPk_by_BlockHash:
    block_hash -> block_height|block_timestamp|prev_hash
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

Secondary indexes like (`script_hash/address/box_id`) are stored as sequence of pointers to a utxo where it was first born.
- `one_to_many` indexes are prefixed with a column family pointer 
- `one_to_one` indexes are prefixed with a column family pointer and a size of of the index value as it is stored here directly.

`Spent_UtxoPk_by_InputPk` and `Spent_InputPk_by_UtxoPk` are used to tell whether box is spent or not.

```
UtxoValueAndUtxoBirthPks_by_UtxoPk:
    utxo_pk -> utxo_value|[utxo_o2m_index_number:utxo_birth_pk,utxo_o2m_index_number:utxo_birth_pk]|[utxo_o2o_index_number:size:utxo_index_value]

Spent_UtxoPk_by_InputPk:
    input_pk -> utxo_pk

Spent_InputPk_by_UtxoPk:
    utxo_pk -> input_pk
```

> Note that indexes are completely generic, `script_hash/address/box_id` are currently selected values but it can be anything from the Utxo.

### Utxo indexes (one-to-one)

As an example, Ergo's output box has a unique identifier `box_id` which we want to search by.

```
UtxoIndex_by_UtxoPk
    box_id -> utxo_pk
```

### Utxo indexes (one-to-many)

We keep secondary indexes like `script_hash/address/etc...` under small-size `utxo_birth_pk` identifiers which is a unique pointer of their creation.
Then we keep relations to all following boxes where given indexed entity occurred. Following table shows 2 example secondary indexes : `script_hash` & `address`.

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

Assets are for now just basic with single `one-to-many` secondary index to search by `asset_id`.
To decide on `spent/unspent`, we can list all assets `asset_birth_pk|asset_pk` and match them with `Spent_UtxoPk_by_InputPk`.

```
AssetValueAndBirthPk_by_UtxoPk:
    utxo_pk -> [asset_value|action|asset_birth_pk]

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