// not needed as the Drop trait works well
pub fn setup_signal_handler(&self, shutdown: ShutdownManager<i32>) {
    let db = Arc::clone(&self.indexer.db_holder.db);
    let is_shutdown = Arc::clone(&self.is_shutdown);
    info!("Starting signal handler thread");
    tokio::task::spawn(async move {
        let shutdown = shutdown.clone();
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to wait for CTRL+C: {}", e);
            std::process::exit(1);
        } else {
            info!("Received interrupt signal");
            if !is_shutdown.load(Ordering::SeqCst) {
                info!("Acquiring db lock for flushing closing...");
                let db_locked = db.write().unwrap();
                db_locked.flush().expect("Failed to flush RocksDB");
                is_shutdown.store(true, Ordering::SeqCst);
                info!("RocksDB successfully flushed and closed.");
            }
            shutdown.trigger_shutdown(0).ok()
        }
    });
}
