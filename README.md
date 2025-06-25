## Chain Indexer

Chain indexer is a universal blockchain indexing tool on top of [redbit](https://github.com/pragmaxim-com/redbit).

Indexer uses tiny `block_height/tx_index/utxo_index/[asset_index]` dictionary pointers to big hashes, ie. not a single hash is duplicated, 
which allows for much better space efficiency and for ~ `6 000 - 12 000 Utxos+Assets / second` syncing speed with local node and an old SSD.

Chain tip is "eventually consistent" through fork competition, ie. forks get settled eventually and superseded forks are deleted from DB.

Currently `Bitcoin`, `Cardano` and `Ergo` are supported.

### Installation (Debian/Ubuntu)

```
sudo apt-get install rustup
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

Indexing might crash especially on laptops with Node running locally and not being synced yet.
In that case, set `fetching_parallelism = "low"` to not put the Node and Laptop under heavy pressure.

### Rest API

http://localhost:8000/swagger-ui/

Querying currently times out during historical indexing. So use it only at the chain tip sync phase 
or when indexing is disabled `indexer.enable = false` and we only run http server to query over existing data.