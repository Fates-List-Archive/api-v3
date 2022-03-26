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
    request: web::Json<models::BotRequest>,
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

    let bot = data.database.get_bot(bot_id).await;

    if bot.is_none() {
        return models::CustomError::NotFoundGeneric.error_response();
    }

    let bot = bot.unwrap();

    let req_data = request.into_inner();

    if req_data.appeal.len() < 7 || req_data.appeal.len() > 4000 {
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: false,
            reason: Some("Appeal length must be between 7 and 4000 characters".to_string()),
            context: None,
        });
    }

    let (title, request_type) = if req_data.request_type == models::BotRequestType::Certification {
        ("Certification Request", "certification")
    } else {
        ("Resubmission", "an appeal")
    };

    if req_data.request_type == models::BotRequestType::Certification {
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
                reason: Some(format!("You cannot certify a bot that has no banner card")),
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
                reason: Some(format!("You cannot certify a bot that has no banner page")),
                context: None,
            });
        }
        if bot.guild_count < 100 {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some(format!("You cannot certify a bot that has fewer than 100 guilds (verified using japi.rest)")),
                context: None,
            });
        }
    }

    let _ = data.config.discord.channels.appeals_channel.send_message(&data.config.discord_http, |m| {
        m.content("<@&".to_string()+&data.config.discord.roles.staff_ping_add_role.clone()+">");
        m.embed(|e| {
            e.url("https://fateslist.xyz/bot/".to_owned()+&bot_id.to_string());
            e.title(title);
            e.color(0x00ff00 as u64);
            e.description(
                format!(
                    "{user} has requested {req_type} for {bot} ({bot_name})\n\n**Please check this bot again!**",
                    user = UserId(user_id as u64).mention(),
                    bot = UserId(bot.user.id.parse::<u64>().unwrap_or(0)).mention(),
                    bot_name = bot.user.username,
                    req_type = request_type,
                )
            );

            e.field("Appeal", req_data.appeal, false);

            e
        });
        m
    }).await;

    return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
        done: true,
        reason: Some("Successfully posted appeal request :)".to_string()),
        context: None,
    });
}
