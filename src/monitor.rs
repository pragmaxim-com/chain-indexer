use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::DateTime;

use crate::{
    api::{BlockHeight, BlockTimestamp, TxCount},
    info,
};

pub struct SimpleBlockMonitor {
    tx_count_report: u64,
    start_time: Instant,
    total_tx_count: Arc<Mutex<usize>>,
}

impl SimpleBlockMonitor {
    pub fn new(tx_count_report: u64) -> Self {
        SimpleBlockMonitor {
            tx_count_report,
            start_time: Instant::now(),
            total_tx_count: Arc::new(Mutex::new(0)),
        }
    }
}
pub trait BlockMonitor<B> {
    fn monitor(&self, block_batch: &Vec<(BlockHeight, B, TxCount, BlockTimestamp)>);
}

impl<Block> BlockMonitor<Block> for SimpleBlockMonitor
where
    Block: std::fmt::Debug,
{
    // THIS CODE IS WRONG, REWRITE !
    fn monitor(&self, block_batch: &Vec<(BlockHeight, Block, TxCount, BlockTimestamp)>) {
        for block in block_batch {
            if (block.0 as u64) % self.tx_count_report == 0 {
                let total_time = self.start_time.elapsed().as_secs();
                let mut total_tx_count = self.total_tx_count.lock().unwrap();
                *total_tx_count += block.2;
                let txs_per_sec = format!("{:.1}", *total_tx_count as f64 / total_time as f64);
                let datetime = DateTime::from_timestamp(block.3, 0).unwrap();
                let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
                info!(
                    "Block @ {} from {} at {} txs/sec, total {}",
                    block.0, readable_date, txs_per_sec, *total_tx_count
                );
            }
        }
    }
}
