use crate::models;
use crate::converters;
use actix_web::http::header::HeaderValue;
/// Handles bot adds
use actix_web::{get, delete, patch, post, web, HttpRequest, HttpResponse, ResponseError};
use log::{error, debug};
use serenity::model::prelude::*;
use std::time::Duration;
use std::collections::HashMap;
use serde_json::json;

/// Simple helper function to check a banner url
pub async fn check_banner_img(
    data: &models::AppState,
    url: &str,
) -> Result<(), models::BannerCheckError> {
    if url.is_empty() {
        return Ok(());
    }

    let req = data
        .requests
        .get(url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(models::BannerCheckError::BadURL)?;

    let status = req.status();

    if !status.is_success() {
        return Err(models::BannerCheckError::StatusError(status.to_string()));
    }

    let default = &HeaderValue::from_str("").unwrap();
    let content_type = req
        .headers()
        .get("Content-Type")
        .unwrap_or(default)
        .to_str()
        .unwrap();

    if content_type.split('/').next().unwrap() != "image" {
        return Err(models::BannerCheckError::BadContentType(
            content_type.to_string(),
        ));
    }

    Ok(())
}

/// Does basic bot checks that are *common* to add and edit bot
/// It is *very* important to note bot.user will only have a valid id, nothing else can be expected
async fn check_bot(
    data: &models::AppState,
    mode: models::BotActionMode,
    bot: &mut models::Bot,
) -> Result<(), models::CheckBotError> {
    let bot_id = bot
        .user
        .id
        .parse::<i64>()
        .map_err(|_| models::CheckBotError::BotNotFound)?;

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
            id = bot
                .client_id
                .parse::<i64>()
                .map_err(|_| models::CheckBotError::BotNotFound)?;
        }

        let resp = data
            .requests
            .get(format!(
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
        let resp_json: models::JAPIApplication = resp
            .json()
            .await
            .map_err(models::CheckBotError::JAPIDeserError)?;
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
            if flag == (models::Flags::EditLocked as i32)
                || flag == (models::Flags::StaffLocked as i32)
            {
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
        } 
        if resolved_vanity.unwrap().target_id != bot.user.id {
            return Err(models::CheckBotError::VanityTaken);
        }
    }

    if let Some(ref invite) = bot.invite {
        if invite.starts_with("P:") {
            let perm_num = invite.split(':').nth(1).unwrap();
            match perm_num.parse::<u64>() {
                Ok(_) => {}
                Err(_) => {
                    return Err(models::CheckBotError::InvalidInvitePermNum);
                }
            }
        }
        debug!("Invite is {}", invite);
        if !invite.replace('"', "").starts_with("https://") {
            debug!("Invite {} is not https", invite);
            return Err(models::CheckBotError::InvalidInvite);
        }
    }

    // Basic checks
    if bot.description.len() > 200 || bot.description.len() < 10 {
        return Err(models::CheckBotError::ShortDescLengthErr);
    }

    if bot.long_description.len() < 200 {
        return Err(models::CheckBotError::LongDescLengthErr);
    }

    bot.long_description = bot.long_description.replace("\\n", "\n").replace("\\r", "");

    if let Some(ref github) = bot.github {
        if !github.replace('"', "").starts_with("https://www.github.com/")
            && !github.replace('"', "").starts_with("https://github.com")
            && !github.is_empty()
        {
            return Err(models::CheckBotError::InvalidGithub);
        }
    }

    if let Some(ref privacy_policy) = bot.privacy_policy {
        if !privacy_policy.replace('"', "").starts_with("https://") && !privacy_policy.is_empty(){
            return Err(models::CheckBotError::InvalidPrivacyPolicy);
        }
    }

    if let Some(ref donate) = bot.donate {
        if !donate.replace('"', "").starts_with("https://") && !donate.is_empty() {
            return Err(models::CheckBotError::InvalidDonate);
        }
    }

    if let Some(ref website) = bot.website {
        if !website.replace('"', "").starts_with("https://") && !website.is_empty() {
            return Err(models::CheckBotError::InvalidWebsite);
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
        check_banner_img(&data, banner)
            .await
            .map_err(models::CheckBotError::BannerCardError)?;
    }
    if let Some(ref banner) = bot.banner_page {
        check_banner_img(&data, banner)
            .await
            .map_err(models::CheckBotError::BannerPageError)?;
    }

    if bot.owners.len() > 5 {
        return Err(models::CheckBotError::OwnerListTooLong);
    }

    let mut done_owners = Vec::new();
    let mut done_owners_lst = Vec::new();

    for owner in bot.owners.clone() {
        if owner.main {
            return Err(models::CheckBotError::MainOwnerAddAttempt);
        }

        let id = owner
            .user
            .id
            .parse::<i64>()
            .map_err(|_| models::CheckBotError::OwnerIDParseError)?;

        if done_owners_lst.contains(&id) {
            continue;
        }

        let user = data.database.get_user(id).await;
        if user.id.is_empty() {
            return Err(models::CheckBotError::OwnerNotFound);
        }

        done_owners.push(models::BotOwner {
            user: user,
            main: false,
        });
        done_owners_lst.push(id);
    }

    bot.owners = done_owners;

    Ok(())
}

/// Add bot
#[post("/users/{id}/bots")]
async fn add_bot(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    bot: web::Json<models::Bot>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(user_id, auth).await {
        let res = check_bot(&data, models::BotActionMode::Add, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string()),
            });
        }
        bot.owners.push(models::BotOwner {
            user: models::User {
                id: user_id.clone().to_string(),
                username: "".to_string(),
                avatar: "".to_string(),
                disc: "0000".to_string(),
                bot: false,
                status: models::Status::Unknown,
            },
            main: true,
        });
        let res = data.database.add_bot(&bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Add bot error".to_string()),
            });
        }
        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.content(
                    "<@&".to_string()
                        + &data.config.discord.roles.staff_ping_add_role.clone()
                        + ">",
                );
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot.user.id);
                    e.title("New Bot!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has added {bot} ({bot_name}) to the queue!",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                    ));

                    e.field("Guild Count (approx)", bot.guild_count.to_string(), true);

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Added bot successfully!".to_string()),
            context: None,
        });
    }
    error!("Add bot auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Edit bot
#[patch("/users/{id}/bots")]
async fn edit_bot(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    bot: web::Json<models::Bot>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data
            .database
            .get_bot(bot.user.id.parse::<i64>().unwrap_or(0))
            .await;
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
                context: None,
            });
        }

        let res = check_bot(&data, models::BotActionMode::Edit, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string()),
            });
        }
        let res = data.database.edit_bot(user_id, &bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Edit bot error".to_string()),
            });
        }
        let result = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot.user.id);
                    e.title("Bot Edit!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has edited {bot} ({bot_name})!",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                    ));

                    e
                });
                m
            })
            .await;

        if result.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(
                    result.unwrap_err().to_string() + " but the bot was edited successfully!",
                ),
                context: Some("Edit bot error".to_string()),
            });
        }

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Edited bot successfully!".to_string()),
            context: None,
        });
    }
    error!("Edit bot auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Transfer ownership
#[patch("/users/{user_id}/bots/{bot_id}/main-owner")]
async fn transfer_ownership(
    req: HttpRequest,
    id: web::Path<models::GetUserBotPath>,
    owner: web::Json<models::BotOwner>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.user_id.clone();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let bot_id = id.bot_id.clone();
    if data.database.authorize_user(user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data.database.get_bot(bot_id).await;
        if bot_user.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }

        let mut got_owner = false;
        for owner in bot_user.clone().unwrap().owners {
            if owner.main && owner.user.id == user_id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(
                    "You are not allowed to transfer ownership of bots you are not main owner of!"
                        .to_string(),
                ),
                context: None,
            });
        }

        // Owner validation
        let owner_copy = owner.clone();

        if !owner_copy.main {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("The main key must be set to 'true'!".to_string()),
                context: None,
            });
        }

        if owner_copy.user.id == user_id.to_string() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You can't transfer ownership to yourself!".to_string()),
                context: None,
            });
        }

        if owner_copy.user.id.parse::<i64>().is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("The user id must be a number fitting in a u64!".to_string()),
                context: None,
            });
        }

        // Does the user actually even exist?
        let owner_user = data
            .database
            .get_user(owner_copy.user.id.parse::<i64>().unwrap())
            .await;
        if owner_user.id.is_empty() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(
                    "The user you wish to transfer ownership to apparently does not exist!"
                        .to_string(),
                ),
                context: None,
            });
        }

        data.database
            .transfer_ownership(user_id, bot_id, owner.clone())
            .await;
        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot_id.to_string());
                    e.title("Bot Ownership Transfer!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has transferred ownership of {bot} ({bot_name}) to {new_owner}!",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot_user.unwrap().user.username,
                        bot = UserId(bot_id as u64).mention(),
                        new_owner = UserId(owner.user.id.parse::<u64>().unwrap_or(0)).mention()
                    ));

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Successfully transferred ownership".to_string()),
            context: None,
        });
    }
    error!("Transfer bot auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Delete bot
#[delete("/users/{user_id}/bots/{bot_id}")]
async fn delete_bot(req: HttpRequest, id: web::Path<models::GetUserBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let user_id = id.user_id;
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let bot_id = id.bot_id;

    if data.database.authorize_user(user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data.database.get_bot(bot_id).await;
        if bot_user.is_none() {
            return models::CustomError::NotFoundGeneric.error_response();
        }

        let mut got_owner = false;
        for owner in bot_user.clone().unwrap().owners {
            if owner.main && owner.user.id == user_id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(
                    "You are not allowed to delete bots you are not main owner of!".to_string(),
                ),
                context: None,
            });
        }

        // Delete the bot
        let res = data.database.delete_bot(user_id, bot_id).await;

        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(
                    "Something went wrong while deleting the bot!".to_string()
                        + &res.unwrap_err().to_string(),
                ),
                context: None,
            });
        }

        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot_id.to_string());
                    e.title("Bot Deleted :(");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has deleted {bot} ({bot_name})",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot_user.unwrap().user.username,
                        bot = UserId(bot_id as u64).mention(),
                    ));

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Successfully transferred ownership".to_string()),
            context: None,
        });
    }
    error!("Delete bot auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

// Get Import Sources
#[get("/import-sources")]
async fn import_sources(_req: HttpRequest) -> HttpResponse {
    return HttpResponse::Ok().json(models::ImportSourceList {
        sources: vec![
            models::ImportSourceListItem {
                id: models::ImportSource::Rdl,
                name: "Rovel Discord List".to_string()
            },
            models::ImportSourceListItem {
                id: models::ImportSource::Topgg,
                name: "Top.gg (ALPHA, may not work reliably)".to_string()
            },
            models::ImportSourceListItem {
                id: models::ImportSource::Ibl,
                name: "Infinity Bot List".to_string()
            },
        ]
    });
}

// Import bots
#[post("/users/{user_id}/bots/{bot_id}/import")]
async fn import_rdl(req: HttpRequest, id: web::Path<models::GetUserBotPath>, src: web::Query<models::ImportQuery>, body: web::Json<models::ImportBody>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = id.user_id;
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        // Fetch bot from RDL
        let bot_id = id.bot_id;

        let mut bot = match src.src {
            models::ImportSource::Rdl => {
                let mut bot_data: HashMap<String, serde_json::Value> = data.requests.get("https://discord.rovelstars.com/api/bots/".to_owned()+&bot_id.to_string())
                .timeout(Duration::from_secs(10))
                .send()
                .await
                .unwrap()
                .json::<HashMap<String, serde_json::Value>>()
                .await
                .unwrap();

                if bot_data.get("err").is_some() {
                    return HttpResponse::BadRequest().json(models::APIResponse {
                        done: false,
                        reason: Some("Bot not found on RDL".to_string()),
                        context: None,
                    });
                }

                debug!("{:?}", bot_data);

                let owners: Vec<String> = bot_data.remove("owners").unwrap().as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();

                let mut extra_owners = Vec::new();

                let mut got_owner = false;
                for owner in owners {
                    if owner == user_id.to_string() {
                        got_owner = true;
                    } else {
                        extra_owners.push(models::BotOwner {
                            user: models::User {
                                id: owner,
                                ..models::User::default()
                            },
                            main: false
                        });
                    }
                }

                if !got_owner {
                    return HttpResponse::BadRequest().json(models::APIResponse {
                        done: false,
                        reason: Some(
                            "You are not allowed to import bots you are not owner of!".to_string(),
                        ),
                        context: None,
                    });
                }

                let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                let website = if website == *"null" || website.is_empty() {
                    None
                } else {
                    Some(website)
                };

                models::Bot {
                    user: models::User {
                        id: bot_id.to_string(),
                        ..models::User::default()
                    },
                    description: bot_data.remove("short").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    long_description: bot_data.remove("desc").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    prefix: Some(bot_data.remove("prefix").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    library: bot_data.remove("lib").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    website,
                    invite: Some(bot_data.remove("invite").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    vanity: "_".to_string() + &bot_data.remove("username").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string() + "-" + &converters::create_token(32),
                    shard_count: 0,
                    owners: extra_owners,
                    tags: vec![
                        // Rovel does not provide us with tags, assert utility
                        models::Tag {
                            id: "utility".to_string(),
                            ..models::Tag::default()
                        }
                    ],
                    ..models::Bot::default()
                }
            }
            models::ImportSource::Topgg => {
                let mut body = body.into_inner();
                let ext_data = &mut body.ext_data;
                if let Some(ref mut bot_data) = ext_data {
                    debug!("{:?}", bot_data);

                    let owners: Vec<String> = bot_data.remove("owners").unwrap().as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
                    
                    let mut extra_owners = Vec::new();

                    let mut got_owner = false;
                    for owner in owners {
                        if owner == user_id.to_string() {
                            got_owner = true;
                        } else {
                            extra_owners.push(models::BotOwner {
                                user: models::User {
                                    id: owner,
                                    ..models::User::default()
                                },
                                main: false
                            });
                        }
                    }    

                    if !got_owner {
                        return HttpResponse::BadRequest().json(models::APIResponse {
                            done: false,
                            reason: Some(
                                "You are not allowed to import bots you are not owner of!".to_string(),
                            ),
                            context: None,
                        });
                    }    

                    let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                    let website = if website == *"null" || website.is_empty() {
                        None
                    } else {
                        Some(website)
                    };   
                    
                    models::Bot {
                        user: models::User {
                            id: bot_id.to_string(),
                            ..models::User::default()
                        },                    
                        vanity: "_".to_string() + &bot_data.remove("username").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string() + "-" + &converters::create_token(32),
                        description: bot_data.remove("shortdesc").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                        long_description: bot_data.remove("longdesc").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                        prefix: Some(bot_data.remove("prefix").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),   
                        invite: Some(bot_data.remove("invite").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()), 
                        shard_count: 0,
                        owners: extra_owners,    
                        website,
                        tags: vec![
                            // top.gg provides tag but they usually don't match and can be arbitary anyways
                            models::Tag {
                                id: "utility".to_string(),
                                ..models::Tag::default()
                            }
                        ],        
                        ..models::Bot::default()
                    }
                } else {
                    return HttpResponse::BadRequest().json(models::APIResponse {
                        done: false,
                        reason: Some("Invalid ext_data".to_string()),
                        context: None,
                    });
                }
            },
            models::ImportSource::Ibl => {
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("Authorization", HeaderValue::from_str(&data.config.secrets.ibl_fates_key).unwrap());

                let mut bot_data: HashMap<String, serde_json::Value> = data.requests.get("https://api.infinitybotlist.com/fates/bots/".to_owned()+&bot_id.to_string())
                .timeout(Duration::from_secs(10))
                .headers(headers)
                .send()
                .await
                .unwrap()
                .json::<HashMap<String, serde_json::Value>>()
                .await
                .unwrap();

                if bot_data.get("message").is_some() {
                    return HttpResponse::BadRequest().json(models::APIResponse {
                        done: false,
                        reason: Some("Bot not found on IBL".to_string()),
                        context: None,
                    });
                }

                debug!("{:?}", bot_data);

                // First check owners
                let main_owner: String = bot_data.remove("owner").unwrap().as_str().unwrap_or("0").to_string();

                let mut got_owner = false;

                let mut extra_owners = Vec::new();

                if main_owner == user_id.to_string() {
                    got_owner = true;
                } 

                // Then additional_owners
                let owners: Vec<String> = bot_data.remove("additional_owners").unwrap().as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
    
                for owner in owners {
                    if owner == user_id.to_string() {
                        got_owner = true;
                    } else {
                        extra_owners.push(models::BotOwner {
                            user: models::User {
                                id: owner,
                                ..models::User::default()
                            },
                            main: false
                        });
                    }
                }    

                if !got_owner {
                    return HttpResponse::BadRequest().json(models::APIResponse {
                        done: false,
                        reason: Some(
                            "You are not allowed to import bots you are not owner of!".to_string(),
                        ),
                        context: None,
                    });
                }

                let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                let website = if website == *"null" || website.is_empty() {
                    None
                } else {
                    Some(website)
                };

                let github = bot_data.remove("github").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                let github = github.replace('"', "").replacen("None", "", 1);
                let github = if github == *"null" || github.is_empty() {
                    None
                } else {
                    Some(github)
                };

                let nsfw = bot_data.remove("nsfw").unwrap_or_else(|| json!(false)).as_bool().unwrap_or(false);

                models::Bot {
                    user: models::User {
                        id: bot_id.to_string(),
                        ..models::User::default()
                    },
                    description: bot_data.remove("short").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    long_description: bot_data.remove("long").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    prefix: Some(bot_data.remove("prefix").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    library: bot_data.remove("library").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    website,
                    github,
                    invite: Some(bot_data.remove("invite").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    vanity: "_".to_string() + &bot_data.remove("name").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string() + "-" + &converters::create_token(32),
                    shard_count: 0,
                    owners: extra_owners,
                    tags: vec![
                        // Rovel does not provide us with tags, assert utility
                        models::Tag {
                            id: "utility".to_string(),
                            ..models::Tag::default()
                        }
                    ],
                    nsfw,
                    ..models::Bot::default()
                }
            },
            _ => {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some("Invalid source".to_string()),
                    context: None,
                });
            }
        };

        let res = check_bot(&data, models::BotActionMode::Add, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string()),
            });
        }
        bot.owners.push(models::BotOwner {
            user: models::User {
                id: user_id.clone().to_string(),
                username: "".to_string(),
                avatar: "".to_string(),
                disc: "0000".to_string(),
                bot: false,
                status: models::Status::Unknown,
            },
            main: true,
        });
        let res = data.database.add_bot(&bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Add bot error".to_string()),
            });
        }
        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.content(
                    "<@&".to_string()
                        + &data.config.discord.roles.staff_ping_add_role.clone()
                        + ">",
                );
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot.user.id);
                    e.title("New Bot!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has added {bot} ({bot_name}) to the queue through {source}!",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention(),
                        source = src.src.source_name()
                    ));

                    e.field("Guild Count (approx)", bot.guild_count.to_string(), true);

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse {
            done: true,
            reason: Some("Added bot successfully!".to_string()),
            context: None,
        });
    }
    error!("Add bot auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}