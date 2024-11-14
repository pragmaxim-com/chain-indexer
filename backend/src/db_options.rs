use rocksdb::{Options, SliceTransform};

pub fn get_db_options(
    disable_autocompaction: bool,
    prefix_extractor_opt: Option<SliceTransform>,
) -> Options {
    let num_cores = num_cpus::get() as i32;

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.increase_parallelism(std::cmp::max(num_cores / 2, 8));
    opts.set_max_background_jobs(std::cmp::max(num_cores / 4, 4));
    opts.set_max_file_opening_threads(std::cmp::max(num_cores / 2, 8));
    opts.set_atomic_flush(false); // Disable atomic flush unless required
    opts.set_db_write_buffer_size(512 * 1024 * 1024); // Adjust DB write buffer
    opts.set_write_buffer_size(256 * 1024 * 1024); // Match buffer size for smaller transactions
    opts.set_max_write_buffer_number(6); // Limit max buffers
    opts.set_min_write_buffer_number_to_merge(2); // Merge smaller buffers
    opts.set_target_file_size_base(128 * 1024 * 1024); // Reduce SSTable size
    opts.set_max_bytes_for_level_base(512 * 1024 * 1024); // Reduce compaction workload
    opts.set_allow_mmap_writes(false);
    opts.set_use_direct_io_for_flush_and_compaction(true); // Use direct I/O for better performance
    opts.set_disable_auto_compactions(disable_autocompaction);
    if let Some(prefix_extractor) = prefix_extractor_opt {
        opts.set_prefix_extractor(prefix_extractor);
    }
    opts
}
