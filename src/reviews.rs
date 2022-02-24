/// Handles reviews
use actix_web::{http, HttpRequest, get, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;
use serenity::model::prelude::*;
use bigdecimal::FromPrimitive;

#[get("/reviews/{id}")]
async fn get_reviews(req: HttpRequest, info: web::Path<models::FetchBotPath>, query: web::Query<models::ReviewQuery>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    if query.page < 1 {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Page must be greater than 0".to_string()),
            context: None,
        });
    }

    let per_page = 9;
    let offset = ((query.page as i64) - 1)*per_page;

    let reviews = data.database.get_reviews(info.id, query.target_type, per_page, offset).await;

    return HttpResponse::Ok().json(models::ParsedReview {
        reviews,
        per_page,
        from: offset,
        average_stars: bigdecimal::BigDecimal::from_i64(0).unwrap(), // TODO
        total: data.database.get_reviews_count(info.id, query.target_type).await,
    });
}