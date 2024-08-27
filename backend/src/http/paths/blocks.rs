use std::ops::Deref;

use actix_web::{web, HttpResponse, Responder};
use model::BlockHash;

use crate::{
    block_read_service::BlockReadService,
    eutxo::{eutxo_families::EutxoFamilies, eutxo_model::EuTx},
};

pub(crate) async fn get_block_by_hash(
    data: web::Json<BlockHash>,
    block_service: web::Data<BlockReadService<EuTx, EutxoFamilies>>,
) -> impl Responder {
    match block_service.get_block_by_hash(&data.into_inner()) {
        Ok(Some(block)) => HttpResponse::Ok().json(block.deref()),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(err) => HttpResponse::from_error(err),
    }
}
