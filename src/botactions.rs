/// Handles bot adds
use actix_web::{http, HttpRequest, get, post, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use std::time::Duration;
use log::error;



/// Simple helper function to check a banner url
pub async fn check_banner_img(data: &models::AppState, url: &str) -> Result<(), models::BannerCheckError> {
    if url.is_empty() {
        return Ok(());
    }
    
    let req = data.requests.get(url)
    .timeout(Duration::from_secs(10))
    .send()
    .await
    .map_err(models::BannerCheckError::BadURL)?;

    let status = req.status();

    if !status.is_success() {
        return Err(models::BannerCheckError::StatusError(status.to_string()));
    }

    let default = &HeaderValue::from_str("").unwrap();
    let content_type = req.headers().get("Content-Type").unwrap_or(default).to_str().unwrap();

    if content_type.split("/").nth(0).unwrap() != "image" {
        return Err(models::BannerCheckError::BadContentType(content_type.to_string()));
    }

    Ok(())
}

/// Does basic bot checks that are *common* to add and edit bot
/// It is *very* important to note bot.user will only have a valid id, nothing else can be expected
async fn check_bot(data: &models::AppState, mode: models::BotActionMode, bot: &mut models::Bot) -> Result<(), models::CheckBotError> {
    let bot_id = bot.user.id.parse::<i64>().map_err(|_| models::CheckBotError::BotNotFound)?;

    // Before doing anything else, get the bot and actually check basic things
    let bot_dat = data.database.get_bot(bot_id).await;
    /* state = await self.db.fetchval("SELECT state FROM bots WHERE bot_id = $1", self.bot_id)
    if state is not None:
        if state in (enums.BotState.denied, enums.BotState.banned):
            return f"This bot has been banned or denied from Fates List.<br/><br/>If you own this bot and wish to appeal this, click <a href='/bot/{self.bot_id}/settings#actions-button-fl'>here</a>"
    */
    if mode == models::BotActionMode::Add {
        if let Some(ref bot_res) = bot_dat {
            if bot_res.state == models::State::Denied || bot_res.state == models::State::Banned {
                return Err(models::CheckBotError::BotBannedOrDenied(bot_res.state));
            } else {
                return Err(models::CheckBotError::AlreadyExists);
            }
        }
        // TODO JAPI checks

        let mut id = bot_id;

        if !bot.client_id.is_empty() {
            id = bot.client_id.parse::<i64>().map_err(|_| models::CheckBotError::BotNotFound)?;
        }

        let resp = data.requests.get(format!(
            "https://japi.rest/discord/v1/application/{bot_id}",
            bot_id = id
        ))
        .timeout(Duration::from_secs(10))
        .header("Authorization", data.config.secrets.japi_key.clone())
        .send()
        .await
        .map_err(models::CheckBotError::JAPIError)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(models::CheckBotError::ClientIDNeeded);
        }
        let resp_json: models::JAPIApplication = resp.json().await.map_err(models::CheckBotError::JAPIDeserError)?;
        if resp_json.data.bot.id != bot_id.to_string() && bot_id.to_string() != bot.client_id {
            return Err(models::CheckBotError::InvalidClientID);
        }
        if !resp_json.data.application.bot_public {
            return Err(models::CheckBotError::PrivateBot);
        }
        bot.guild_count = resp_json.data.bot.approximate_guild_count;
    } else {
        if let Some(ref bot_res) = bot_dat {
            if !bot_res.client_id.is_empty() && bot_res.client_id != bot.client_id {
                return Err(models::CheckBotError::ClientIDImmutable);
            }
        }
    }

    if bot.prefix.clone().unwrap_or_else(|| "".to_string()).len() > 9 {
        return Err(models::CheckBotError::PrefixTooLong);
    }

    if bot.vanity.len() < 2 {
        return Err(models::CheckBotError::NoVanity);
    }
    let resolved_vanity = data.database.resolve_vanity(&bot.vanity).await;

    if resolved_vanity.is_some() {
        if mode == models::BotActionMode::Add {
            return Err(models::CheckBotError::VanityTaken);
        } else {
            if resolved_vanity.unwrap().target_id != bot.user.id {
                return Err(models::CheckBotError::VanityTaken);
            }
        }
    }

    if let Some(ref invite) = bot.invite {
        if invite.starts_with("P:") {
            let perm_num = invite.split(":").nth(1).unwrap();
            match perm_num.parse::<u64>() {
                Ok(_) => {
                }
                Err(_) => {
                    return Err(models::CheckBotError::InvalidInvitePermNum);
                }
            }
        } else {
            if !invite.starts_with("https://") {
                return Err(models::CheckBotError::InvalidInvite);
            }
        }
    }

    // Basic checks
    if bot.description.len() > 200 || bot.description.len() < 10 {
        return Err(models::CheckBotError::ShortDescLengthErr);
    }

    if bot.long_description.len() < 200 {
        return Err(models::CheckBotError::LongDescLengthErr);
    }

    if let Some(ref github) = bot.github {
        if !github.starts_with("https://www.github.com/") && !github.starts_with("https://github.com") {
            return Err(models::CheckBotError::InvalidGithub);
        }
    }

    if let Some(ref privacy_policy) = bot.privacy_policy {
        if !privacy_policy.starts_with("https://") {
            return Err(models::CheckBotError::InvalidPrivacyPolicy);
        }
    }

    if let Some(ref donate) = bot.donate {
        if !donate.starts_with("https://") {
            return Err(models::CheckBotError::InvalidDonate);
        }
    }

    let bot_user = data.database.get_user(bot_id).await;

    if bot_user.id.is_empty() {
        return Err(models::CheckBotError::BotNotFound);
    }

    // Fill in actual bot id so our code can assume it
    bot.user = bot_user;

    // Tags
    if bot.tags.len() > 10 {
        return Err(models::CheckBotError::TooManyTags);
    }

    let full_tags = data.database.bot_list_tags().await;
    let mut tag_list = Vec::new();

    for tag in bot.tags.clone() {
        if full_tags.contains(&tag) {
            tag_list.push(tag)
        }
    }

    bot.tags = tag_list;
    
    if bot.tags.is_empty() {
        return Err(models::CheckBotError::NoTags);
    }

    // Features
    if bot.features.len() > 5 {
        return Err(models::CheckBotError::TooManyFeatures);
    }

    if !bot.features.is_empty() {
        let full_features = data.database.bot_features().await;
        let mut feature_list = Vec::new();

        for feature in bot.features.clone() {
            if full_features.contains(&feature) {
                feature_list.push(feature)
            }
        }

        bot.features = feature_list;
    }

    // Banner
    if let Some(ref banner) = bot.banner_card {
        check_banner_img(&data, banner).await.map_err(models::CheckBotError::BannerCardError)?;
    }
    if let Some(ref banner) = bot.banner_page {
        check_banner_img(&data, banner).await.map_err(models::CheckBotError::BannerPageError)?;
    }

    Ok(())
}

/// Add bot
#[post("/users/{id}/bots")]
async fn add_bot(req: HttpRequest, id: web::Path<models::FetchBotPath>, bot: web::Json<models::Bot>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let res = check_bot(&data, models::BotActionMode::Add, &mut bot.into_inner()).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string())
            });
        } else {
            return HttpResponse::Ok().json(models::APIResponse {
                done: true,
                reason: Some("Check success".to_string()),
                context: None
            });
        }
    } else {
        error!("Add bot auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}
