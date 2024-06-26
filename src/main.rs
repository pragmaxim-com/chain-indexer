mod api;
mod btc;
mod codec;
mod logger;
mod rocks;
mod syncer;

use clap::{Arg, ArgAction, Command};
use std::{env, ops::Deref, sync::Arc};
use {
    api::ChainSyncer,
    btc::{btc_client::BtcClient, btc_processor::BtcProcessor},
    rocks::rocks_storage::{RocksStorage, ADDRESS_CF, CACHE_CF, META_CF},
};

fn cli() -> Command {
    Command::new("indexBTC")
        .about("Bitcoin transactions indexer")
        .version("1.0")
        .author("Pragmaxim <pragmaxim@gmail.com>")
        .args([
            Arg::new("db-path")
                .long("db-path")
                .allow_hyphen_values(true)
                .require_equals(true)
                .action(ArgAction::Set)
                .num_args(1)
                .default_value("/tmp/index_btc")
                .help("Absolute path to db directory"),
            Arg::new("btc-url")
                .long("btc-url")
                .action(ArgAction::Set)
                .require_equals(true)
                .allow_hyphen_values(true)
                .num_args(1)
                .default_value("http://127.0.0.1:8332")
                .help("Url of local bitcoin-core"),
            Arg::new("db-engine")
                .long("db-engine")
                .action(ArgAction::Set)
                .require_equals(true)
                .allow_hyphen_values(true)
                .num_args(1)
                .default_value("rocks-db")
                .help("rocks-db or sled-db"),
        ])
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let matches = cli().get_matches();

    let bitcoin_url = matches.get_one::<String>("btc-url").unwrap();
    log!("Connecting to bitcoin-core at : {}", bitcoin_url);

    let db_path = matches.get_one::<String>("db-path").unwrap();
    log!("Using db path : {}", db_path);

    let num_cores = num_cpus::get();
    log!("Number of CPU cores: {}", num_cores);

    let db_engine = matches
        .get_one::<String>("db-engine")
        .map(|s| s.deref())
        .unwrap();
    log!("Using db engine : {}", db_engine);
    let full_db_path = format!("{}/{}", db_path, db_engine);
    let (username, password) = match (
        env::var("BITCOIN_RPC_USERNAME"),
        env::var("BITCOIN_RPC_PASSWORD"),
    ) {
        (Ok(user), Ok(pass)) => (user, pass),
        _ => {
            panic!("Error: Bitcoin RPC BITCOIN_RPC_PASSWORD or BITCOIN_RPC_USERNAME environment variable not set");
        }
    };

    let client = Arc::new(BtcClient::new(bitcoin_url, &username, &password));

    let processor = Arc::new(BtcProcessor {});

    let storage = RocksStorage::new(
        num_cores as i32,
        &full_db_path,
        vec![ADDRESS_CF, CACHE_CF, META_CF],
    )
    .unwrap();

    let syncer = ChainSyncer::new(client, processor, Arc::new(storage));
    syncer.sync(844566, 50).await;
    Ok(())
}
