use actix_web::{web, HttpResponse, Responder};

use crate::{
    block_read_service::BlockReadService,
    eutxo::{eutxo_families::EutxoFamilies, eutxo_model::EuTx},
    model::BlockHash,
};

pub(crate) async fn get_block_by_hash(
    data: web::Json<BlockHash>,
    block_service: web::Data<BlockReadService<EuTx, EutxoFamilies>>,
) -> impl Responder {
    HttpResponse::Ok().body("Data retrieved successfully")
}
