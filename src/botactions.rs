/// Handles bot actions (view, add, edit, delete, transfer)

use crate::models;
use crate::converters;
use actix_web::http::header::HeaderValue;
use actix_web::{get, delete, patch, post, web, http, web::Json, HttpRequest, HttpResponse};
use log::{error, debug};
use serenity::model::prelude::*;
use std::time::Duration;
use std::collections::HashMap;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

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
            } 
            return Err(models::CheckBotError::AlreadyExists);
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
        } else if !invite.replace('"', "").starts_with("https://") {
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

    let mut total_links = 0;
    let mut links_rendered = 0;

    for (key, value) in bot.extra_links.iter() {
        if key.len() > 20 {
            return Err(models::CheckBotError::ExtraLinkKeyTooLong);
        }
        if value.len() > 200 {
            return Err(models::CheckBotError::ExtraLinkValueTooLong);
        }

        total_links += 1;

        if key.starts_with('_') {
            continue;
        }

        links_rendered += 1;

        if !value.starts_with("https://") {
            return Err(models::CheckBotError::ExtraLinkValueNotHTTPS);
        }
    }

    if links_rendered > 10 {
        return Err(models::CheckBotError::ExtraLinksTooManyRendered);
    } else if total_links > 20 {
        return Err(models::CheckBotError::ExtraLinksTooMany);
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
                feature_list.push(feature);
            }
        }

        bot.features = feature_list;
    }

    // Banner
    if let Some(ref banner) = bot.banner_card {
        check_banner_img(data, banner)
            .await
            .map_err(models::CheckBotError::BannerCardError)?;
    }
    if let Some(ref banner) = bot.banner_page {
        check_banner_img(data, banner)
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
            user,
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
    let auth_default = &HeaderValue::from_str("").unwrap();
    
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(id.id, auth).await {
        let res = check_bot(&data, models::BotActionMode::Add, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&res.unwrap_err())); 
        }
        bot.owners.push(models::BotOwner {
            user: models::User {
                id: id.id.to_string(),
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
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::SQLError(res.unwrap_err()))); 
        }

        // Metro Code
        let mut map = json!(
            {
                "bot_id": bot.user.id,
                "username": &bot.user.username,
                "banner": &bot.banner_card,
                "owner": id.id,
                "extra_owners": bot.owners.clone().into_iter().map(|x| x.user.id).collect::<Vec<String>>(),
                "description": &bot.description,
                "long_description": &bot.long_description,
                "tags": &bot.tags.clone().into_iter().map(|x| x.id).collect::<Vec<String>>(),
                "library": &bot.library,
                "nsfw": bot.flags.contains(&(models::Flags::NSFW as i32)),
                "prefix": &bot.prefix,
            }
        );

        let obj: &mut serde_json::Map<std::string::String, serde_json::Value> = map.as_object_mut().unwrap();

        if bot.extra_links.contains_key("Website") {
            obj.insert("website".to_string(), json!(bot.extra_links["Website"]));
        } else if bot.extra_links.contains_key("website") {
            obj.insert("website".to_string(), json!(bot.extra_links["website"]));
        }

        if bot.extra_links.contains_key("Donate") {
            obj.insert("donate".to_string(), json!(bot.extra_links["Donate"]));
        } else if bot.extra_links.contains_key("donate") {
            obj.insert("donate".to_string(), json!(bot.extra_links["donate"]));
        }

        if bot.extra_links.contains_key("Github") {
            obj.insert("github".to_string(), json!(bot.extra_links["Github"]));
        } else if bot.extra_links.contains_key("github") {
            obj.insert("github".to_string(), json!(bot.extra_links["github"]));
        }

        let metro = data.database.requests.post("https://catnip.metrobots.xyz/bots?list_id=5800d395-beb3-4d79-90b9-93e1ca674b40")
        .header("Authorization", &data.config.secrets.metro_key)
        .json(&map)
        .send()
        .await;    

        if let Ok(m) = metro {
            debug!("Metro code: {}", m.text().await.unwrap());
        } else {
            error!("Metro code: error {}", metro.unwrap_err());
        }
        // End of metro code

        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.content(data.config.discord.roles.staff_ping_add_role.mention());
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot.user.id);
                    e.title("New Bot!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has added {bot} ({bot_name}) to the queue!",
                        user = UserId(id.id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                    ));

                    e.field("Guild Count (approx)", bot.guild_count.to_string(), true);

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

/// Edit bot
#[patch("/users/{id}/bots")]
async fn edit_bot(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    bot: web::Json<models::Bot>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    let mut bot = bot.into_inner();
    if data.database.authorize_user(id.id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let owners = data
            .database
            .get_bot_owners(bot.user.id.parse::<i64>().unwrap_or(0))
            .await;

        let mut got_owner = false;
        for owner in owners {
            if owner.user.id == id.id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::Forbidden));
        }

        let res = check_bot(&data, models::BotActionMode::Edit, &mut bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&res.unwrap_err()));
        }
        let res = data.database.edit_bot(id.id, &bot).await;
        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::SQLError(res.unwrap_err()))); 
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
                        user = UserId(id.id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention()
                    ));

                    e
                });
                m
            })
            .await;

        if result.is_err() {
            error!("Error sending message: {}", result.unwrap_err());
            return HttpResponse::Ok().json(models::APIResponse::ok());
        }

        // Invalidate the cache
        data.database.bot_cache.invalidate(&id.id).await;

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

/// Transfer ownership
#[patch("/users/{user_id}/bots/{bot_id}/main-owner")]
async fn transfer_ownership(
    req: HttpRequest,
    id: web::Path<models::GetUserBotPath>,
    owner: web::Json<models::BotOwner>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(id.user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let owners = data
            .database
            .get_bot_owners(id.bot_id)
            .await;

        let mut got_owner = false;
        for bot_owner in owners {
            if bot_owner.main && bot_owner.user.id == id.user_id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::CheckBotError::NotMainOwner));
        }

        // Owner validation
        let owner_copy = owner.clone();

        if !owner_copy.main {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&models::GenericError::InvalidFields));
        }

        if owner_copy.user.id == id.user_id.to_string() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&models::GenericError::InvalidFields));
        }

        if owner_copy.user.id.parse::<i64>().is_err() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&models::GenericError::InvalidFields));
        }

        // Does the user actually even exist?
        let owner_user = data
            .database
            .get_user(owner_copy.user.id.parse::<i64>().unwrap())
            .await;
        if owner_user.id.is_empty() {
            return HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound));
        }

        data.database
            .transfer_ownership(id.user_id, id.bot_id, owner.clone())
            .await;
        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &id.bot_id.to_string());
                    e.title("Bot Ownership Transfer!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has transferred ownership of {bot} to {new_owner}!",
                        user = UserId(id.user_id as u64).mention(),
                        bot = UserId(id.bot_id as u64).mention(),
                        new_owner = UserId(owner.user.id.parse::<u64>().unwrap_or(0)).mention()
                    ));

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

/// Delete bot
#[delete("/users/{user_id}/bots/{bot_id}")]
async fn delete_bot(req: HttpRequest, id: web::Path<models::GetUserBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();

    if data.database.authorize_user(id.user_id, auth).await {
        // Before doing anything else, get the bot from db and check if user is owner
        let bot_user = data.database.get_bot(id.bot_id).await;
        if bot_user.is_none() {
            return HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound));
        }

        let mut got_owner = false;
        for owner in bot_user.clone().unwrap().owners {
            if owner.main && owner.user.id == id.user_id.to_string() {
                got_owner = true;
                break;
            }
        }

        if !got_owner {
            return HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::CheckBotError::NotMainOwner));
        }

        // Delete the bot
        let res = data.database.delete_bot(id.user_id, id.bot_id).await;

        if res.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::SQLError(res.unwrap_err()))); 
        }

        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &id.bot_id.to_string());
                    e.title("Bot Deleted :(");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has deleted {bot} ({bot_name})",
                        user = UserId(id.user_id as u64).mention(),
                        bot_name = bot_user.unwrap().user.username,
                        bot = UserId(id.bot_id as u64).mention(),
                    ));

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

// Get Import Sources
#[get("/import-sources")]
async fn import_sources(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(models::ImportSourceList {
        sources: vec![
            models::ImportSourceListItem {
                id: models::ImportSource::Rdl,
                name: "Rovel Discord List".to_string()
            },
            models::ImportSourceListItem {
                id: models::ImportSource::Ibl,
                name: "Infinity Bot List".to_string()
            },
            models::ImportSourceListItem {
                id: models::ImportSource::Custom,
                name: "Custom Source (top.gg etc.)".to_string()
            },
        ]
    })
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

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Lightleap-Dest", HeaderValue::from_str("Fates List").unwrap());
        headers.insert("Lightleap-Site", HeaderValue::from_str("https://fateslist.xyz").unwrap());

        let mut bot = match src.src {
            models::ImportSource::Rdl => {
                let mut bot_data: HashMap<String, serde_json::Value> = data.requests.get("https://discord.rovelstars.com/api/bots/".to_owned()+&bot_id.to_string())
                .timeout(Duration::from_secs(10))
                .headers(headers)
                .send()
                .await
                .unwrap()
                .json::<HashMap<String, serde_json::Value>>()
                .await
                .unwrap();

                if bot_data.get("err").is_some() {
                    return HttpResponse::NotFound().json(models::APIResponse::err_small(&models::GenericError::NotFound));
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

                let mut extra_links = indexmap::IndexMap::new();

                let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                if website != *"null" && !website.is_empty() {
                    extra_links.insert("Website".to_string(), website);
                };

                let github = bot_data.remove("github").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                if github != *"null" && !github.is_empty() {
                    extra_links.insert("Github".to_string(), github);
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
                    extra_links,
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
            models::ImportSource::Custom => {
                let mut body = body.into_inner();
                let ext_data = &mut body.ext_data;
                if let Some(ref mut bot_data) = ext_data {
                    debug!("{:?}", bot_data);

                    let owners: Vec<String> = bot_data.remove("owners").unwrap_or_else(|| json!([])).as_array().unwrap_or(&Vec::new()).iter().map(|x| x.as_str().unwrap_or_default().to_string()).collect();
                    
                    let mut extra_owners = Vec::new();
                    
                    let mut got_owner = false;

                    if owners.is_empty() {
                        got_owner = true
                    } else {
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


                    let mut extra_links = indexmap::IndexMap::new();

                    let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                    if website != *"null" && !website.is_empty() {
                        extra_links.insert("Website".to_string(), website);
                    };
    
                    let github = bot_data.remove("github").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                    if github != *"null" && !github.is_empty() {
                        extra_links.insert("Github".to_string(), github);
                    };    
                    
                    models::Bot {
                        user: models::User {
                            id: bot_id.to_string(),
                            ..models::User::default()
                        },                    
                        vanity: "_".to_string() + &bot_data.remove("username").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string() + "-" + &converters::create_token(32),
                        description: bot_data.remove("description").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                        long_description: bot_data.remove("long_description").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                        prefix: Some(bot_data.remove("prefix").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),   
                        invite: Some(bot_data.remove("invite").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()), 
                        shard_count: 0,
                        owners: extra_owners,    
                        extra_links,
                        tags: vec![
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
                        reason: Some("Invalid bot data".to_string()),
                        context: None,
                    });
                }
            },
            models::ImportSource::Ibl => {
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("Authorization", HeaderValue::from_str(&data.config.secrets.ibl_fates_key).unwrap());
                headers.insert("Lightleap-Dest", HeaderValue::from_str("Fates List").unwrap());
                headers.insert("Lightleap-Site", HeaderValue::from_str("https://fateslist.xyz").unwrap());



                let mut bot_data: HashMap<String, serde_json::Value> = data.requests.get("https://api.infinitybotlist.com/fates/bots/".to_owned()+&bot_id.to_string())
                .timeout(Duration::from_secs(10))
                .headers(headers)
                .send()
                .await
                .unwrap()
                .json::<HashMap<String, serde_json::Value>>()
                .await
                .unwrap();

                let ibl_msg = bot_data.get("message");

                if ibl_msg.is_some() {
                    return HttpResponse::NotFound().json(models::APIResponse::err_small(&models::GenericError::NotFound));
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

                let mut extra_links = indexmap::IndexMap::new();

                let website = bot_data.remove("website").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                if website != *"null" && !website.is_empty() {
                    extra_links.insert("Website".to_string(), website);
                };

                let github = bot_data.remove("github").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string();
                if github != *"null" && !github.is_empty() {
                    extra_links.insert("Github".to_string(), github);
                };

                let nsfw = bot_data.remove("nsfw").unwrap_or_else(|| json!(false)).as_bool().unwrap_or(false);

                let mut flags = Vec::new();

                if nsfw {
                    flags.push(models::Flags::NSFW as i32);
                }

                models::Bot {
                    user: models::User {
                        id: bot_id.to_string(),
                        ..models::User::default()
                    },
                    description: bot_data.remove("short").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    long_description: bot_data.remove("long").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    prefix: Some(bot_data.remove("prefix").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    library: bot_data.remove("library").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string(),
                    extra_links,
                    invite: Some(bot_data.remove("invite").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string()),
                    vanity: "_".to_string() + &bot_data.remove("name").unwrap_or_else(|| json!("")).as_str().unwrap_or("").to_string() + "-" + &converters::create_token(32),
                    shard_count: 0,
                    owners: extra_owners,
                    flags,
                    tags: vec![
                        // Rovel does not provide us with tags, assert utility
                        models::Tag {
                            id: "utility".to_string(),
                            ..models::Tag::default()
                        }
                    ],
                    ..models::Bot::default()
                }
            },
            _ => {
                return HttpResponse::NotFound().json(models::APIResponse::err_small(&models::GenericError::NotFound));
            }
        };

        let res = check_bot(&data, models::BotActionMode::Add, &mut bot).await;
        if res.is_err() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&res.unwrap_err()));
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
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::SQLError(res.unwrap_err()))); 
        }
        let _ = data
            .config
            .discord
            .channels
            .bot_logs
            .send_message(&data.config.discord_http, |m| {
                m.content(data.config.discord.roles.staff_ping_add_role.mention());
                m.embed(|e| {
                    e.url("https://fateslist.xyz/bot/".to_owned() + &bot.user.id);
                    e.title("New Bot!");
                    e.color(0x00ff00 as u64);
                    e.description(format!(
                        "{user} has added {bot} ({bot_name}) to the queue through {source}!",
                        user = UserId(user_id as u64).mention(),
                        bot_name = bot.user.username,
                        bot = UserId(bot.user.id.parse::<u64>().unwrap()).mention(),
                        source = if src.src == models::ImportSource::Custom {
                            src.src.source_name() + "(" + &src.custom_source.clone().unwrap_or_else(|| "Unknown".to_string()) + ")"
                        } else {
                            src.src.source_name()
                        }
                    ));

                    e.field("Guild Count (approx)", bot.guild_count.to_string(), true);

                    e
                });
                m
            })
            .await;

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

/// Post Stats
#[post("/bots/{id}/stats")]
async fn post_stats(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    stats: web::Json<models::BotStats>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot_id = id.id;

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_bot(bot_id, auth).await {
        // Firstly make sure user does not have the StatsLocked flag
        let bot = data.database.get_bot(bot_id).await.unwrap();

        if converters::flags_check(&bot.flags, vec![models::Flags::StatsLocked as i32]) {
            return HttpResponse::BadRequest().json(models::APIResponse::err_small(&models::GenericError::APIBan("StatsLocked".to_string())));
        }

        let resp = data.database.post_stats(bot_id, bot.client_id.parse().unwrap_or(0), stats.into_inner(), &data.config.secrets.japi_key).await;
        match resp {
            Ok(()) => HttpResponse::build(http::StatusCode::OK).json(models::APIResponse::ok()),
            Err(err) => {
                HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&err))
            }
        }
    } else {
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
    }
}

// Get Bot
#[get("/bots/{id}")]
async fn get_bot(req: HttpRequest, id: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let id = id.into_inner();

    if req.headers().contains_key("Frostpaw") {
        let auth_default = &HeaderValue::from_str("").unwrap();
        let auth = req.headers().get("Frostpaw-Auth").unwrap_or(auth_default);
        let mut event_user: Option<String> = None;
        if !auth.clone().is_empty() {
            let auth_bytes = auth.to_str();
            match auth_bytes {
                Ok(auth_str) => {
                    let auth_split = auth_str.split('|');
                    let auth_vec = auth_split.collect::<Vec<&str>>();

                    let user_id = auth_vec.get(0).unwrap_or(&"");
                    let token = auth_vec.get(1).unwrap_or(&"");

                    let user_id_str = (*user_id).to_string();

                    let user_id_i64 = user_id_str.parse::<i64>().unwrap_or(0);

                    if data
                        .database
                        .authorize_user(user_id_i64, token.as_ref())
                        .await
                    {
                        event_user = Some(user_id_str);
                    }
                }
                Err(err) => {
                    error!("{}", err);
                }
            }
        }

        let event = models::Event {
            m: models::EventMeta {
                e: models::EventName::BotView,
                eid: Uuid::new_v4().to_hyphenated().to_string(),
            },
            ctx: models::EventContext {
                target: id.id.to_string(),
                target_type: models::TargetType::Bot,
                user: event_user,
                ts: chrono::Utc::now().timestamp(),
            },
            props: models::BotViewProp {
                vote_page: req.headers().contains_key("Frostpaw-Vote-Page"),
                widget: false,
                invite: req.headers().contains_key("Frostpaw-Invite"),
            },
        };
        data.database.ws_event(event).await;
    }

    if req.headers().contains_key("Frostpaw-Invite") {
        data.database.update_bot_invite_amount(id.id).await;
    }

    // Check bot cache
    let cache = data.database.bot_cache.get(&id.id);
    
    match cache {
        Some(bot) => {
            debug!("Bot cache hit for {}", id.id);
            HttpResponse::Ok().json(bot)
        },
        None => {
            debug!("Bot cache miss for {}", id.id);
            let bot = data.database.get_bot(id.id).await;
            match bot {
                Some(bot_data) => {
                    let bot_data = Arc::new(bot_data);
                    data.database.bot_cache.insert(id.id, bot_data.clone()).await;
                    HttpResponse::Ok().json(bot_data)
                },
                _ => HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound)),
            }
        }
    }
}

// Get Random Bot
#[get("/random-bot")]
async fn random_bot(req: HttpRequest) -> Json<models::IndexBot> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot = data.database.random_bot().await;
    Json(bot)
}

/// Get Bot Settings
#[get("/users/{user_id}/bots/{bot_id}/settings")]
async fn get_bot_settings(
    req: HttpRequest,
    info: web::Path<models::GetUserBotPath>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.user_id;

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let resp = data.database.get_bot_settings(info.bot_id).await;
        match resp {
            Ok(bot) => {
                // Check if in owners before returning
                for owner in &bot.owners {
                    let id = owner.user.id.parse::<i64>().unwrap_or(0);
                    if id == user_id {
                        return HttpResponse::build(http::StatusCode::OK).json(
                            models::BotSettings {
                                bot,
                                context: models::BotSettingsContext {
                                    tags: data.database.bot_list_tags().await,
                                    features: data.database.bot_features().await,
                                },
                            },
                        );
                    }
                }
                HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
            }
            Err(err) => {
                HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                    done: false,
                    reason: Some(err.to_string()),
                    context: None,
                })
            }
        }
    } else {
        error!("Bot Settings Auth error");
        HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
    }
}