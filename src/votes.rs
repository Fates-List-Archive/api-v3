use crate::models;
use crate::converters;
use actix_web::http::header::HeaderValue;
use actix_web::{get, patch, web, http, HttpRequest, HttpResponse, ResponseError};
use log::error;


/// Create Bot Vote
#[patch("/users/{user_id}/bots/{bot_id}/votes")]
async fn vote_bot(
    req: HttpRequest,
    info: web::Path<models::GetUserBotPath>,
    vote: web::Query<models::VoteBotQuery>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.user_id;
    let bot_id = info.bot_id;

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let bot = data.database.get_bot(bot_id).await;
        if bot.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }
        let bot = bot.unwrap();
        if converters::flags_check(&bot.flags, vec![models::Flags::System as i32]) {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You can't vote for system bots!".to_string()),
                context: None,
            });
        }
        let res = data.database.vote_bot(user_id, bot_id, vote.test).await;
        if res.is_err() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: None,
            });
        }
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse::ok());
    }
    error!("Vote Bot Auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Create Server Vote
#[patch("/users/{user_id}/servers/{server_id}/votes")]
async fn vote_server(
    req: HttpRequest,
    info: web::Path<models::GetUserServerPath>,
    vote: web::Query<models::VoteBotQuery>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.user_id;
    let server_id = info.server_id;

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let server = data.database.get_server(server_id).await;
        if server.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }
        let server = server.unwrap();
        if converters::flags_check(&server.flags, vec![models::Flags::System as i32]) {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You can't vote for system servers!".to_string()),
                context: None,
            });
        }
        let res = data
            .database
            .vote_server(
                &data.config.discord_http_server,
                user_id,
                server_id,
                vote.test,
            )
            .await;
        if res.is_err() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: None,
            });
        }
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse::ok());
    }
    error!("Vote Server Auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Bot: Has User Voted?
#[get("/users/{user_id}/bots/{bot_id}/votes")]
async fn has_user_bot_voted(req: HttpRequest, info: web::Path<models::GetUserBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let user_flags = data.database.get_user_flags(info.user_id).await;

    if user_flags.contains(&models::UserFlags::VotesPrivate) {
        return HttpResponse::build(http::StatusCode::OK).json(models::UserVoted {
            vote_right_now: true,
            ..models::UserVoted::default()
        })
    }

    let resp = data.database.get_user_bot_voted(info.bot_id, info.user_id).await;
    HttpResponse::build(http::StatusCode::OK).json(resp)
}

/// Server: Has User Voted?
#[get("/users/{user_id}/servers/{server_id}/votes")]
async fn has_user_server_voted(req: HttpRequest, info: web::Path<models::GetUserServerPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    
    let user_flags = data.database.get_user_flags(info.user_id).await;

    if user_flags.contains(&models::UserFlags::VotesPrivate) {
        return HttpResponse::build(http::StatusCode::OK).json(models::UserVoted {
            vote_right_now: true,
            ..models::UserVoted::default()
        })
    }
    
    let resp = data.database.get_user_server_voted(info.server_id, info.user_id).await;
    HttpResponse::build(http::StatusCode::OK).json(resp)
}