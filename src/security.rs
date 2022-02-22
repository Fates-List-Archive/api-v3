/// Endpoints to manage security related features such as token regeneration
use actix_web::{http, HttpRequest, delete, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;

/// Issues (regenerates) a new bot token
#[delete("/bots/{id}/token")]
async fn new_bot_token(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if data.database.authorize_bot(bot_id, auth).await {
        data.database.new_bot_token(bot_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated bot token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}

/// Issues (regenerates) a new bot token
#[delete("/users/{id}/token")]
async fn new_user_token(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if data.database.authorize_user(user_id, auth).await {
        data.database.new_user_token(user_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated user token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}