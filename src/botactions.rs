/// Handles bot adds
use actix_web::{http, HttpRequest, post, patch, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use std::time::Duration;
use log::error;
use serenity::model::prelude::*;

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

        for flag in bot.flags.clone() {
            if flag == (models::Flags::EditLocked as i32) || flag == (models::Flags::StaffLocked as i32) {
                return Err(models::CheckBotError::EditLocked);
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
    let mut tag_list_raw = Vec::new();

    for tag in bot.tags.clone() {
        if full_tags.contains(&tag) && !tag_list_raw.contains(&tag.id) {
            tag_list.push(tag.clone());
            tag_list_raw.push(tag.id);
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

    if bot.owners.len() > 5 {
        return Err(models::CheckBotError::OwnerListTooLong);
    }

    let mut done_owners = Vec::new();
    let mut done_owners_lst = Vec::new();

    for owner in bot.owners.clone() {
        if owner.main {
            return Err(models::CheckBotError::MainOwnerAddAttempt)
        }

        let id = owner.user.id.parse::<i64>().map_err(|_| models::CheckBotError::OwnerIDParseError)?;
        
        if done_owners_lst.contains(&id) {
            continue
        }

        let user = data.database.get_user(id).await;
        if user.id.is_empty() {
            return Err(models::CheckBotError::OwnerNotFound);
        }

        done_owners.push(models::BotOwner {
            user: user,
            main: false
        });
        done_owners_lst.push(id);
    }

    bot.owners = done_owners;

    Ok(())
}

/// Add bot
#[post("/users/{id}/bots")]
async fn add_bot(req: HttpRequest, id: web::Path<models::FetchBotPath>, bot: web::Json<models::Bot>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(user_id, auth).await {
        let res = check_bot(&data, models::BotActionMode::Add, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string())
            });
        } else {
            bot.owners.push(models::BotOwner {
                user: models::User {
                    id: user_id.clone().to_string(),
                    username: "".to_string(),
                    avatar: "".to_string(),
                    disc: "0000".to_string(),
                    bot: false
                },
                main: true,
            });
            let res = data.database.add_bot(bot.clone()).await;
            if res.is_err() {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some(res.unwrap_err().to_string()),
                    context: Some("Add bot error".to_string())
                });    
            } else {
                let _ = data.config.discord.channels.bot_logs.send_message(&data.config.discord_http, |m| {
                    m.content("<@&".to_string()+&data.config.discord.roles.staff_ping_add_role.clone()+">");
                    m.embed(|e| {
                        e.url("https://fateslist.xyz/bot/".to_owned()+&bot.user.id);
                        e.title("New Bot!");
                        e.color(0x00ff00 as u64);
                        e.description(
                            format!(
                                "{user} has added {bot} ({bot_name}) to the queue!",
                                user = UserId(user_id as u64).mention(),
                                bot_name = bot.user.username,
                                bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                            )
                        );

                        e.field("Guild Count (approx)", bot.guild_count.to_string(), true);

                        e
                    });
                    m
                }).await;

                return HttpResponse::Ok().json(models::APIResponse {
                    done: true,
                    reason: Some("Check success: ".to_string() + &bot.guild_count.to_string()),
                    context: None
                });
            }
        }
    } else {
        error!("Add bot auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}

/// Edit bot
#[patch("/users/{id}/bots")]
async fn edit_bot(req: HttpRequest, id: web::Path<models::FetchBotPath>, bot: web::Json<models::Bot>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data.database.get_bot(bot.user.id.parse::<i64>().unwrap_or(0)).await;
        if bot_user.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        } 

        let mut got_owner = false;
        for owner in bot_user.unwrap().owners {
            if owner.user.id == user_id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You are not allowed to edit this bot!".to_string()),
                context: None
            });
        }
       
        let res = check_bot(&data, models::BotActionMode::Edit, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string())
            });
        } else {
            let res = data.database.edit_bot(user_id, bot.clone()).await;
            if res.is_err() {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some(res.unwrap_err().to_string()),
                    context: Some("Edit bot error".to_string())
                });    
            } else {
                let _ = data.config.discord.channels.bot_logs.send_message(&data.config.discord_http, |m| {
                    m.embed(|e| {
                        e.url("https://fateslist.xyz/bot/".to_owned()+&bot.user.id);
                        e.title("Bot Edit!");
                        e.color(0x00ff00 as u64);
                        e.description(
                            format!(
                                "{user} has editted {bot} ({bot_name})!",
                                user = UserId(user_id as u64).mention(),
                                bot_name = bot.user.username,
                                bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                            )
                        );

                        e
                    });
                    m
                }).await;

                return HttpResponse::Ok().json(models::APIResponse {
                    done: true,
                    reason: Some("Check success: ".to_string() + &bot.guild_count.to_string()),
                    context: None
                });
            }
        }
    } else {
        error!("Edit bot auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}

/// Edit bot
#[patch("/users/{user_id}/bots/{bot_id}/main-owner")]
async fn transfer_ownership(req: HttpRequest, id: web::Path<models::GetUserBotPath>, owner: web::Json<models::BotOwner>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.user_id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    let bot_id = id.bot_id.clone();
    if data.database.authorize_user(user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data.database.get_bot(bot_id).await;
        if bot_user.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }

        let mut got_owner = false;
        for owner in bot_user.clone().unwrap().owners {
            if owner.main && owner.user.id == user_id.to_string(){
                got_owner = true;
                break;
            } 
        }

        if !got_owner {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You are not allowed to transfer ownership of bots you are not main owner of!".to_string()),
                context: None
            });
        }
       
        // Owner validation
        let owner_copy = owner.clone();

        if !owner_copy.main {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("The main key must be set to 'true'!".to_string()),
                context: None
            });
        }

        if owner_copy.user.id == user_id.to_string() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You can't transfer ownership to yourself!".to_string()),
                context: None
            });
        }

        if owner_copy.user.id.parse::<i64>().is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("The user id must be a number fitting in a u64!".to_string()),
                context: None
            });
        }

        // Does the user actually even exist?
        let owner_user = data.database.get_user(owner_copy.user.id.parse::<i64>().unwrap()).await;
        if owner_user.id.is_empty() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("The user you wish to transfer ownership to apparently does not exist!".to_string()),
                context: None
            });
        }

            let res = data.database.transfer_ownership(user_id, bot_id, owner.clone()).await;
            let _ = data.config.discord.channels.bot_logs.send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned()+&bot_id.to_string());
                    e.title("Bot Ownership Transfer!");
                    e.color(0x00ff00 as u64);
                    e.description(
                        format!(
                            "{user} has transferred ownership of {bot} ({bot_name}) to {new_owner}!",
                            user = UserId(user_id as u64).mention(),
                            bot_name = bot_user.unwrap().user.username,
                            bot = UserId(bot_id as u64).mention(),
                            new_owner = UserId(owner.user.id.parse::<u64>().unwrap_or(0)).mention()
                        )
                    );

                    e
                });
                m
            }).await;

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Successfully transferred ownership".to_string()),
            context: None
        });

    } else {
        error!("Add bot auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}
