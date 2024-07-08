use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::DateTime;

use crate::{
    api::{BlockMonitor, TxCount},
    eutxo::eutxo_api::EuBlock,
    info,
};

pub struct EuBlockMonitor {
    min_tx_count_report: usize,
    start_time: Instant,
    total_and_last_report_tx_count: Arc<Mutex<(usize, usize)>>,
}

impl EuBlockMonitor {
    pub fn new(min_tx_count_report: usize) -> Self {
        EuBlockMonitor {
            min_tx_count_report,
            start_time: Instant::now(),
            total_and_last_report_tx_count: Arc::new(Mutex::new((0, 0))),
        }
    }
}

impl BlockMonitor<EuBlock> for EuBlockMonitor {
    fn monitor(&self, block_batch: &Vec<EuBlock>, tx_count: TxCount) {
        let mut total_tx_count = self.total_and_last_report_tx_count.lock().unwrap();
        let new_total_tx_count = total_tx_count.0 + tx_count;
        if new_total_tx_count > total_tx_count.1 + self.min_tx_count_report {
            *total_tx_count = (new_total_tx_count, new_total_tx_count);
            let last_block = block_batch.last().unwrap();
            let total_time = self.start_time.elapsed().as_secs();
            let txs_per_sec = format!("{:.1}", new_total_tx_count as f64 / total_time as f64);
            let datetime = DateTime::from_timestamp(last_block.timestamp, 0).unwrap();
            let readable_date = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
            info!(
                "Block @ {} from {} at {} txs/sec, total {}",
                last_block.height, readable_date, txs_per_sec, new_total_tx_count
            );
        } else {
            *total_tx_count = (new_total_tx_count, total_tx_count.1);
        }
    }
}
