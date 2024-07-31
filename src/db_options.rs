use rocksdb::Options;

use crate::info;

pub fn get_db_options() -> Options {
    let num_cores = num_cpus::get() as i32;
    info!("Number of CPU cores: {}", num_cores);

    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.increase_parallelism(num_cores); // Set this based on your CPU cores
    opts.set_max_background_jobs(std::cmp::max(num_cores / 2, 6));
    opts.set_max_file_opening_threads(std::cmp::max(num_cores, 16));
    opts.set_atomic_flush(true);
    opts.set_db_write_buffer_size(256 * 1024 * 1024); // flush atomically whole db
    opts.set_write_buffer_size(1024 * 1024 * 1024); // 1GB - this is ignored by lower size of set_db_write_buffer_size
    opts.set_max_write_buffer_number(8);
    opts.set_min_write_buffer_number_to_merge(4);
    opts.set_target_file_size_base(256 * 1024 * 1024); // 256 MB
    opts.set_max_bytes_for_level_base(2048 * 1024 * 1024); // 2GB for compaction
    opts.set_use_direct_io_for_flush_and_compaction(true);
    opts.set_disable_auto_compactions(true);

    // opts.set_level_compaction_dynamic_level_bytes(true);
    // opts.set_max_subcompactions(num_cores as u32 / 2);
    // opts.set_allow_mmap_reads(true);

    // opts.set_compaction_style(rocksdb::DBCompactionStyle::Level);
    // opts.set_allow_concurrent_memtable_write(false);
    // opts.set_memtable_factory(MemtableFactory::Vector);
    // opts.set_allow_mmap_writes(true); // cannot be used together with use_direct_io_for_flush_and_compaction

    opts
}
