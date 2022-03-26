// Endpoints to get and modify users
use crate::models;
use actix_web::http::header::HeaderValue;
use actix_web::{get, patch, web, HttpRequest, HttpResponse, ResponseError};
use log::error;

#[get("/profiles/{id}")]
async fn get_profile(req: HttpRequest, info: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let profile = data.database.get_profile(info.id).await;

    if let Some(profile) = profile {
        return HttpResponse::Ok().json(profile);
    }
    return HttpResponse::NotFound().json(models::APIResponse {
        done: false,
        reason: Some("Profile not found".to_string()),
        context: Some("Profile not found".to_string()),
    });
}

#[patch("/profiles/{id}")]
async fn update_profile(
    req: HttpRequest,
    info: web::Path<models::FetchBotPath>,
    body: web::Json<models::Profile>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.id;

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let profile = data.database.get_profile(user_id).await;
        if profile.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }
        let profile = profile.unwrap();

        if profile.state == models::UserState::ProfileEditBan {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You have been banned from using this API endpoint".to_string()),
                context: Some("Profile edit ban".to_string()),
            });
        }

        let res = data
            .database
            .update_profile(user_id, body.into_inner())
            .await;

        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Profile update error".to_string()),
            });
        }
        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Updated profile successfully!".to_string()),
            context: None,
        });
    } else {
        error!("Update profile auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}
