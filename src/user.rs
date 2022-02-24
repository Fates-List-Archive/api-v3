// Endpoints to get and modify users
use actix_web::{HttpRequest, get, web, HttpResponse, ResponseError};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;


#[get("/profiles/{id}")]
async fn get_profile(req: HttpRequest, info: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let profile = data.database.get_profile(info.id).await;

    if let Some(profile) = profile {
        return HttpResponse::Ok().json(profile);
    } else {
        return HttpResponse::NotFound().json(models::APIResponse {
            done: false,
            reason: Some("Profile not found".to_string()),
            context: Some("Profile not found".to_string())
        });
    }
}