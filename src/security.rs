/// Endpoints to manage security related features such as token regeneration

use crate::models;
use actix_web::http::header::HeaderValue;
use actix_web::{delete, http, web, HttpRequest, HttpResponse};
use log::error;

/// Issues (regenerates) a new bot token
#[delete("/bots/{id}/token")]
async fn new_bot_token(req: HttpRequest, id: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot_id = id.id;
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_bot(bot_id, auth).await {
        data.database.new_bot_token(bot_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated bot token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err(&models::GenericError::Forbidden))
    }
}

/// Issues (regenerates) a new user token
#[delete("/users/{id}/token")]
async fn new_user_token(req: HttpRequest, id: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id;
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        data.database.new_user_token(user_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated user token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err(&models::GenericError::Forbidden))
    }
}

/// Revokes a clients auth
#[delete("/users/{id}/frostpaw/clients/{client_id}")]
async fn revoke_client(req: HttpRequest, id: web::Path<models::UserClientAuth>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id;
    let client_id = id.client_id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        data.database.revoke_client(user_id, client_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated user token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err(&models::GenericError::Forbidden))
    }
}


/// Issues (regenerates) a new server token
#[delete("/servers/{id}/token")]
async fn new_server_token(req: HttpRequest, id: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let server_id = id.id;
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_server(server_id, auth).await {
        data.database.new_server_token(server_id).await;
        HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: true,
            reason: Some("Successfully regenerated server token".to_string()),
            context: None,
        })
    } else {
        error!("Token auth error");
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err(&models::GenericError::Forbidden))
    }
}
