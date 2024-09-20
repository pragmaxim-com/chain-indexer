use std::{ops::Deref, sync::Arc};

use actix_web::{web, HttpResponse, Responder};

use crate::{
    block_read_service::BlockReadService,
    eutxo::{eutxo_families::EutxoFamilies, eutxo_model::EuTx},
};
use std::str::FromStr;

pub(crate) async fn get_block_by_hash_height_or_latest(
    path: web::Path<String>,
    block_service: web::Data<Arc<BlockReadService<EuTx, EutxoFamilies>>>,
) -> impl Responder {
    let identifier = path.into_inner();

    // Check if the identifier is "latest"
    if identifier == "latest" {
        match block_service.get_latest_block() {
            Ok(Some(block)) => return HttpResponse::Ok().json(block.deref()),
            Ok(None) => return HttpResponse::NotFound().finish(),
            Err(err) => return HttpResponse::from_error(err),
        }
    }

    // Try parsing the identifier as a block height
    if let Ok(height) = u32::from_str(&identifier) {
        match block_service.get_block_by_height(height.into()) {
            Ok(Some(block)) => return HttpResponse::Ok().json(block.deref()),
            Ok(None) => return HttpResponse::NotFound().finish(),
            Err(err) => return HttpResponse::from_error(err),
        }
    }

    // If it's not a height, treat it as a block hash
    match block_service.get_block_by_hash(&identifier.into()) {
        Ok(Some(block)) => HttpResponse::Ok().json(block.deref()),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::from_error(err),
    }
}
