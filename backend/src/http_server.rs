use std::sync::Arc;

use actix_web::{dev::Server, web, App, HttpServer};
use redb::Database;
use crate::settings::HttpSettings;

pub(crate) fn run(http_conf: HttpSettings, db: Arc<Database>) -> Server {
    unsafe {std::env::set_var("RUST_LOG", "info")};
    env_logger::init();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&db)))
    })
    .bind(http_conf.bind_address.clone())
    .unwrap()
    .run()
}
