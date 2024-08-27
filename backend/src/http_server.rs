use std::sync::Arc;

use actix_web::{dev::Server, web, App, HttpServer};

use crate::{
    block_read_service::BlockReadService, http::paths, rocks_db_batch::CustomFamilies,
    settings::HttpSettings,
};

pub(crate) fn run<Tx: Send + Sync + 'static, CF: CustomFamilies + Send + Sync + 'static>(
    http_conf: HttpSettings,
    block_service: Arc<BlockReadService<Tx, CF>>,
) -> Server {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&block_service)))
            .route("/blocks", web::post().to(paths::blocks::get_block_by_hash))
    })
    .bind(http_conf.bind_address.clone())
    .unwrap()
    .run()
}
