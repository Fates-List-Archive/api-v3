// Endpoints to get and modify users
use crate::models;
use actix_web::http::header::HeaderValue;
use actix_web::{get, put, patch, web, http, HttpRequest, HttpResponse};
use log::error;

/// Gets a user given a id
#[get("/blazefire/{id}")]
pub async fn get_user_from_id(req: HttpRequest, info: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::Ok().json(data.database.get_user(info.id).await)
}

/// Gets a users perms given a id
#[get("/baypaw/perms/{id}")]
pub async fn get_user_perms(req: HttpRequest, info: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    // This api may crash with 408 if baypaw is down, it merely proxies
    let req = data.database.requests.get(
    format!(
        "http://127.0.0.1:1234/perms/{id}",
        id = info.id
    ))
    .send()
    .await
    .unwrap();

    HttpResponse::Ok().json(req.json::<bristlefrost::StaffPerm>().await.unwrap())
}


#[get("/profiles/{id}")]
async fn get_profile(req: HttpRequest, info: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let profile = data.database.get_profile(info.id).await;

    if let Some(profile) = profile {
        return HttpResponse::Ok().json(profile);
    }
    HttpResponse::NotFound().json(models::APIResponse {
        done: false,
        reason: Some("Profile not found".to_string()),
        context: Some("Profile not found".to_string()),
    })
}

#[patch("/profiles/{id}")]
async fn update_profile(
    req: HttpRequest,
    info: web::Path<models::FetchBotPath>,
    body: web::Json<models::Profile>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(info.id, auth).await {
        let profile = data.database.get_profile(info.id).await;
        if profile.is_none() {
            return HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound));
        }
        let profile = profile.unwrap();

        if profile.state == models::UserState::ProfileEditBan {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::APIBan("ProfileEditBan".to_string())));
        }

        if body.flags.contains(&(models::UserFlags::VotesPrivate as i32)) && !profile.user_experiments.contains(&models::UserExperiments::UserVotePrivacy) {
                return models::UserExperiments::UserVotePrivacy.not_enabled();
        }

        let res = data
            .database
            .update_profile(info.id, body.into_inner())
            .await;

        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&res.unwrap_err()));
        }
        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    error!("Update profile auth error");
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

#[put("/profiles/{id}/old-roles")]
async fn recieve_profile_roles(
    req: HttpRequest,
    info: web::Path<models::FetchBotPath>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(info.id, auth).await {
        let profile = data.database.get_profile(info.id).await;
        if profile.is_none() {
            return HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound));
        }
        let profile = profile.unwrap();

        let rl = data.database.get_ratelimit(models::Ratelimit::RoleUpdate, info.id).await;

        if rl.is_some() && rl.unwrap() > 0 {
            return HttpResponse::BadRequest().json(models::APIResponse::rl(rl.unwrap()));
        }    

        if profile.state == models::UserState::ProfileEditBan {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::APIBan("ProfileEditBan".to_string())));
        }

        data.database.set_ratelimit(models::Ratelimit::RoleUpdate, info.id).await;

        let update = data
            .database
            .update_user_bot_roles(info.id, &data.config.discord)
            .await;

        if update.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&update.unwrap_err()));
        }

        return HttpResponse::Ok().json(update.unwrap());
    }
    error!("Update profile auth error");
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}
