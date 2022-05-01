use crate::models;
use actix_web::http::header::HeaderValue;
/// Handles bot appeals
use actix_web::{http, post, web, HttpRequest, HttpResponse, ResponseError};
use log::error;
use serenity::model::prelude::*;

#[post("/users/{user_id}/bots/{bot_id}/appeal")]
async fn appeal_bot(
    req: HttpRequest,
    info: web::Path<models::GetUserBotPath>,
    request: web::Json<models::Appeal>,
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
    if !data.database.authorize_user(user_id, auth).await {
        error!("Appeal Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    let rl = data.database.get_ratelimit(models::Ratelimit::Appeal, user_id).await;

    if rl.is_some() && rl.unwrap() > 0 {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: true,
            reason: Some(format!("Please wait {} seconds before retrying this appeal!", rl.unwrap())),
            context: Some("Ratelimit".to_string()),
        })
    }

    let bot = data.database.get_bot(bot_id).await;

    if bot.is_none() {
        return models::CustomError::NotFoundGeneric.error_response();
    }

    let bot = bot.unwrap();

    let req_data = request.into_inner();

    if req_data.request_type == models::AppealType::Report {
        let user_experiments = data.database.get_user_experiments(user_id).await;

        if !user_experiments.contains(&models::UserExperiments::BotReport) {
            return models::UserExperiments::BotReport.not_enabled();
        }
    }

    if req_data.appeal.len() < 7 || req_data.appeal.len() > 4000 {
        return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
            done: false,
            reason: Some("Appeal length must be between 7 and 4000 characters".to_string()),
            context: None,
        });
    }

    let (request_field, title, request_type) = if req_data.request_type == models::AppealType::Certification {
        ("Reason/What's Unique?", "Certification Request", "certification")
    } else if req_data.request_type == models::AppealType::Appeal {
        ("Appeal", "Resubmission", "an appeal")
    } else {
        ("Report", "Report", "a staff member to look into a report on")
    };

    if req_data.request_type == models::AppealType::Certification {
        if bot.state != models::State::Approved {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot appeal a bot that is not approved".to_string()),
                context: None,
            });
        }
        if bot.banner_card.is_none()
            || !bot
                .banner_card
                .unwrap_or_else(|| "".to_string())
                .starts_with("https://")
        {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a bot that has no banner card".to_string()),
                context: None,
            });
        }
        if bot.banner_page.is_none()
            || !bot
                .banner_page
                .unwrap_or_else(|| "".to_string())
                .starts_with("https://")
        {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a bot that has no banner page".to_string()),
                context: None,
            });
        }
        if bot.guild_count < 100 {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a bot that has fewer than 100 guilds (verified using japi.rest)".to_string()),
                context: None,
            });
        }
    }

    data.database.set_ratelimit(models::Ratelimit::Appeal, user_id).await;

    let msg = data.config.discord.channels.appeals_channel.send_message(&data.config.discord_http, |m| {
        m.content(data.config.discord.roles.staff_ping_add_role.mention());

        m.embed(|e| {
            e.url("https://fateslist.xyz/bot/".to_owned()+&bot_id.to_string());
            e.title(title);
            e.color(0x00ff00);
            e.description(
                format!(
                    "{user} has requested {req_type} for {bot} ({bot_name})\n\n**Please check this bot again!**",
                    user = UserId(user_id as u64).mention(),
                    bot = UserId(bot.user.id.parse::<u64>().unwrap_or(0)).mention(),
                    bot_name = bot.user.username,
                    req_type = request_type,
                )
            );

            e.field(request_field, req_data.appeal, false);

            e
        });
        m
    }).await;

    if msg.is_err() {
        return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
            done: false,
            reason: Some("Failed to send appeal message. Please try again.".to_string()),
            context: None,
        });
    }

    HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
        done: true,
        reason: Some("Successfully posted appeal request :)".to_string()),
        context: None,
    })
}

#[post("/users/{user_id}/servers/{server_id}/appeal")]
async fn appeal_server(
    req: HttpRequest,
    info: web::Path<models::GetUserServerPath>,
    request: web::Json<models::Appeal>,
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
    if !data.database.authorize_user(user_id, auth).await {
        error!("Appeal Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    let server = data.database.get_server(server_id).await;

    if server.is_none() {
        return models::CustomError::NotFoundGeneric.error_response();
    }

    let server = server.unwrap();

    let req_data = request.into_inner();

    let user_experiments = data.database.get_user_experiments(user_id).await;

    if req_data.request_type == models::AppealType::Report { 
        if !user_experiments.contains(&models::UserExperiments::BotReport) {
            return models::UserExperiments::BotReport.not_enabled();
        }
    } else if !user_experiments.contains(&models::UserExperiments::ServerAppealCertification) {
        return models::UserExperiments::ServerAppealCertification.not_enabled();
    }

    if req_data.appeal.len() < 7 || req_data.appeal.len() > 4000 {
        return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
            done: false,
            reason: Some("Appeal length must be between 7 and 4000 characters".to_string()),
            context: None,
        });
    }

    let (request_field, title, request_type) = if req_data.request_type == models::AppealType::Certification {
        ("Reason/What's Unique?", "Certification Request", "certification")
    } else if req_data.request_type == models::AppealType::Appeal {
        ("Appeal", "Resubmission", "an appeal")
    } else {
        ("Report", "Report", "a staff member to look into a report on")
    };

    if req_data.request_type == models::AppealType::Certification {
        if server.state != models::State::Approved {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot appeal a server that is not approved".to_string()),
                context: None,
            });
        }
        if server.banner_card.is_none()
            || !server
                .banner_card
                .unwrap_or_else(|| "".to_string())
                .starts_with("https://")
        {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a server that has no banner card".to_string()),
                context: None,
            });
        }
        if server.banner_page.is_none()
            || !server
                .banner_page
                .unwrap_or_else(|| "".to_string())
                .starts_with("https://")
        {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a server that has no banner page".to_string()),
                context: None,
            });
        }
        if server.guild_count < 100 {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some("You cannot certify a server that has fewer than 100 members".to_string()),
                context: None,
            });
        }
    }

    let msg = data.config.discord.channels.appeals_channel.send_message(&data.config.discord_http, |m| {
        m.content(data.config.discord.roles.staff_ping_add_role.mention());

        m.embed(|e| {
            e.url("https://fateslist.xyz/server/".to_owned()+&server_id.to_string());
            e.title(title);
            e.color(0x00ff00);
            e.description(
                format!(
                    "{user} has requested {req_type} for server {server} ({server_name})\n\n**Please check this bot again!**",
                    user = UserId(user_id as u64).mention(),
                    server = server.user.id,
                    server_name = server.user.username,
                    req_type = request_type,
                )
            );

            e.field(request_field, req_data.appeal, false);

            e
        });
        m
    }).await;

    if msg.is_err() {
        return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
            done: false,
            reason: Some("Failed to send appeal message. Please try again.".to_string()),
            context: None,
        });
    }

    HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
        done: true,
        reason: Some("Successfully posted appeal request :)".to_string()),
        context: None,
    })
}
