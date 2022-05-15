use crate::models;
use actix_web::{http, get, post, web, HttpRequest, HttpResponse};
use actix_web::http::header::HeaderValue;

#[get("/notifications/info")]
async fn get_notif_info(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).json(models::NotificationInfo {
        public_key: data.config.secrets.notif_public_key.clone()
    })
}

#[post("/notifications/{id}/sub")]
pub async fn subscribe(
    req: HttpRequest, 
    id: web::Path<models::FetchBotPath>,
    notif: web::Json<models::NotificationSub>
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let auth_default = &HeaderValue::from_str("").unwrap();
    
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(id.id, auth).await {
        let notif_res = data.database.subscribe_notifs(id.id, notif.into_inner()).await;

        if notif_res.is_ok() {
            return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse::ok());
        } else if let Err(e) = notif_res {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&e));
        }
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

#[get("/notifications/{id}/test")]
pub async fn test_notifs(
    req: HttpRequest, 
    id: web::Path<models::FetchBotPath>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let auth_default = &HeaderValue::from_str("").unwrap();
    
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(id.id, auth).await {
        data.database.test_notifs(id.id).await;
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}