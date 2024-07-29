use std::{cell::RefCell, sync::Arc, time::Instant};

use crate::{
    api::BlockMonitor,
    info,
    model::{Block, TxCount},
};

use super::eutxo_model::EuTx;

pub struct EuBlockMonitor {
    min_tx_count_report: usize,
    start_time: Instant,
    total_and_last_report_tx_count: Arc<RefCell<(usize, usize)>>,
}

impl EuBlockMonitor {
    pub fn new(min_tx_count_report: usize) -> Self {
        EuBlockMonitor {
            min_tx_count_report,
            start_time: Instant::now(),
            total_and_last_report_tx_count: Arc::new(RefCell::new((0, 0))),
        }
    }
}

impl BlockMonitor<EuTx> for EuBlockMonitor {
    fn monitor(&self, block_batch: &Vec<Block<EuTx>>, tx_count: &TxCount) {
        let mut total_tx_count = self.total_and_last_report_tx_count.borrow_mut();
        let new_total_tx_count = total_tx_count.0 + tx_count;
        if new_total_tx_count > total_tx_count.1 + self.min_tx_count_report {
            *total_tx_count = (new_total_tx_count, new_total_tx_count);
            let last_block = block_batch.last().unwrap();
            let total_time = self.start_time.elapsed().as_secs();
            let txs_per_sec = format!("{:.1}", new_total_tx_count as f64 / total_time as f64);
            info!(
                "{} Blocks @ {} from {} at {} txs/sec, total {}",
                block_batch.len(),
                last_block.header.height,
                last_block.header.timestamp,
                txs_per_sec,
                new_total_tx_count
            );
        } else {
            *total_tx_count = (new_total_tx_count, total_tx_count.1);
        }
    }
}
