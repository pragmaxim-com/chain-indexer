use std::{cell::RefCell, time::Instant};

use crate::model::{BatchWeight, Block};
use crate::{api::BlockMonitor, info};

use super::eutxo_model::EuTx;

pub struct EuBlockMonitor {
    min_weight_report: usize,
    start_time: Instant,
    total_and_last_report_weight: RefCell<(usize, usize)>,
}

impl EuBlockMonitor {
    pub fn new(min_tx_count_report: usize) -> Self {
        EuBlockMonitor {
            min_weight_report: min_tx_count_report,
            start_time: Instant::now(),
            total_and_last_report_weight: RefCell::new((0, 0)),
        }
    }
}

impl BlockMonitor<EuTx> for EuBlockMonitor {
    fn monitor(&self, block_batch: &[Block<EuTx>], batch_weight: &BatchWeight) {
        let mut total_weight = self.total_and_last_report_weight.borrow_mut();
        let new_total_weight = total_weight.0 + batch_weight;
        if new_total_weight > total_weight.1 + self.min_weight_report {
            *total_weight = (new_total_weight, new_total_weight);
            let last_block = block_batch.last().unwrap();
            let total_time = self.start_time.elapsed().as_secs();
            let txs_per_sec = format!("{:.1}", new_total_weight as f64 / total_time as f64);
            info!(
                "{} Blocks @ {} from {} at {} ins+outs+assets per second, total {}",
                block_batch.len(),
                last_block.header.height,
                last_block.header.timestamp,
                txs_per_sec,
                new_total_weight
            );
        } else {
            *total_weight = (new_total_weight, total_weight.1);
        }
    }
}
