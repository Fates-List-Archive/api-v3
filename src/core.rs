// A core endpoint is one that is absolutely essential for proper list functions
use crate::converters;
use crate::models;
use actix_web::http::header::HeaderValue;
use actix_web::{get, http, patch, post, web, web::Json, HttpRequest, HttpResponse, ResponseError};
use log::error;
use uuid::Uuid;
use strum::IntoEnumIterator;

#[get("/index")]
async fn index(req: HttpRequest, info: web::Query<models::IndexQuery>) -> Json<models::Index> {
    let mut index = models::Index::new();

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    if info.target_type.as_ref().unwrap_or(&"bot".to_string()) == "bot" {
        let cache = data.database.get_index_bots_from_cache().await;

        if cache.is_some() {
            return Json(cache.unwrap());
        }

        index.top_voted = data.database.index_bots(models::State::Approved).await;
        index.certified = data.database.index_bots(models::State::Certified).await;
        index.tags = data.database.bot_list_tags().await;
        index.new = data.database.index_new_bots().await;
        index.features = data.database.bot_features().await;

        data.database.set_index_bots_to_cache(&index).await;
    } else {
        let cache = data.database.get_index_servers_from_cache().await;

        if cache.is_some() {
            return Json(cache.unwrap());
        }

        index.top_voted = data.database.index_servers(models::State::Approved).await;
        index.certified = data.database.index_servers(models::State::Certified).await;
        index.new = data.database.index_new_servers().await;
        index.tags = data.database.server_list_tags().await;

        data.database.set_index_servers_to_cache(&index).await;
    }
    Json(index)
}

#[get("/code/{vanity}")]
async fn get_vanity(req: HttpRequest, code: web::Path<String>) -> HttpResponse {
    if code.starts_with('_') {
        return models::CustomError::NotFoundGeneric.error_response();
    }
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let resolved_vanity = data.database.resolve_vanity(&code.into_inner()).await;
    match resolved_vanity {
        Some(data) => HttpResponse::build(http::StatusCode::OK).json(data),
        _ => models::CustomError::NotFoundGeneric.error_response(),
    }
}

// Docs template
#[get("/_docs_template")]
async fn docs_tmpl(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).body(data.docs.clone())
}

// Enum Docs template
#[get("/_enum_docs_template")]
async fn enum_docs_tmpl(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).body(data.enum_docs.clone())
}

// Experiment List
#[get("/experiments")]
async fn experiments(_req: HttpRequest) -> HttpResponse {
    let mut exp_map = Vec::new();
    for exp in models::UserExperiments::iter() {
        exp_map.push(models::UserExperimentListItem {
            name: exp.to_string(),
            value: exp,
        });
    }

    HttpResponse::build(http::StatusCode::OK).json(models::ExperimentList {
        user_experiments: exp_map,
    })
}

// Policies
#[get("/policies")]
async fn policies(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).json(&data.config.policies)
}

// Partners
#[get("/partners")]
async fn partners(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).json(&data.config.partners)
}

// Bot route
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

                    let user_id_str = user_id.to_string();

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

    let cached_bot = data.database.get_bot_from_cache(id.id).await;
    match cached_bot {
        Some(bot) => HttpResponse::build(http::StatusCode::OK).json(bot),
        None => {
            let bot = data.database.get_bot(id.id).await;
            match bot {
                Some(bot_data) => HttpResponse::build(http::StatusCode::OK).json(bot_data),
                _ => models::CustomError::NotFoundGeneric.error_response(),
            }
        }
    }
}

// Server route
#[get("/servers/{id}")]
async fn get_server(req: HttpRequest, id: web::Path<models::FetchBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let id = id.into_inner();

    let mut event_user: Option<String> = None;

    if req.headers().contains_key("Frostpaw") {
        let auth_default = &HeaderValue::from_str("").unwrap();
        let auth = req.headers().get("Frostpaw-Auth").unwrap_or(auth_default);
        if !auth.clone().is_empty() {
            let auth_bytes = auth.to_str();
            match auth_bytes {
                Ok(auth_str) => {
                    let auth_split = auth_str.split('|');
                    let auth_vec = auth_split.collect::<Vec<&str>>();

                    let user_id = auth_vec.get(0).unwrap_or(&"");
                    let token = auth_vec.get(1).unwrap_or(&"");

                    let user_id_str = user_id.to_string();

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
                e: models::EventName::ServerView,
                eid: Uuid::new_v4().to_hyphenated().to_string(),
            },
            ctx: models::EventContext {
                target: id.id.to_string(),
                target_type: models::TargetType::Server,
                user: event_user.clone(),
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

    let mut invite_link: Option<String> = None;

    // Server invite handling using GUILDINVITE ipc
    if req.headers().contains_key("Frostpaw-Invite") {
        let invite_link_result = data
            .database
            .resolve_guild_invite(
                id.id,
                event_user
                    .unwrap_or_else(|| "0".to_string())
                    .parse::<i64>()
                    .unwrap_or(0),
            )
            .await;

        if invite_link_result.is_err() {
            return HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                done: false,
                reason: Some(invite_link_result.unwrap_err().to_string()),
                context: None,
            });
        }
        invite_link = Some(invite_link_result.unwrap());
        data.database.update_server_invite_amount(id.id).await;
    }

    let cached_server = data.database.get_server_from_cache(id.id).await;
    match cached_server {
        Some(mut server) => {
            server.invite_link = invite_link;
            HttpResponse::build(http::StatusCode::OK).json(server)
        }
        None => {
            let server = data.database.get_server(id.id).await;
            match server {
                Some(mut server_data) => {
                    server_data.invite_link = invite_link;
                    HttpResponse::build(http::StatusCode::OK).json(server_data)
                }
                _ => models::CustomError::NotFoundGeneric.error_response(),
            }
        }
    }
}

/// Search route. Uses PUT because request body
#[get("/search")]
async fn search(req: HttpRequest, info: web::Query<models::SearchQuery>) -> Json<models::Search> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let search = info.into_inner();

    let cached_resp = data.database.get_search_from_cache(&search).await;
    match cached_resp {
        Some(resp) => Json(resp),
        None => {
            let search_resp = data.database.search(search).await;
            Json(search_resp)
        }
    }
}

// Search Tags
#[get("/search-tags")]
async fn search_tags(
    req: HttpRequest,
    info: web::Query<models::SearchTagQuery>,
) -> Json<models::Search> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let search_resp = data.database.search_tags(&info.q).await;
    Json(search_resp)
}

// Get Random Bot

#[get("/random-bot")]
async fn random_bot(req: HttpRequest) -> Json<models::IndexBot> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot = data.database.random_bot().await;
    Json(bot)
}

// Get Random Server

#[get("/random-server")]
async fn random_server(req: HttpRequest) -> Json<models::IndexBot> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let server = data.database.random_server().await;
    Json(server)
}

/// Bot: Has User Voted?
#[get("/users/{user_id}/bots/{bot_id}/votes")]
async fn has_user_voted(req: HttpRequest, info: web::Path<models::GetUserBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.user_id;
    let bot_id = info.bot_id;

    let resp = data.database.get_user_voted(bot_id, user_id).await;
    HttpResponse::build(http::StatusCode::OK).json(resp)
}

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
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: false,
            reason: Some(
r#"Successfully voted for this bot!

<em>Pro Tip</em>: You can vote for bots directly on your server using Fates List Helper. Fates List Helper
also supports vote reminders as well!

You can invite Fates List Helper to your server by <a style="color: blue !important" href="https://discord.com/api/oauth2/authorize?client_id=811073947382579200&permissions=2048&scope=bot%20applications.commands">clicking here</a>!

If you have previously invited Squirrelflight, please remove and add Fates List Helper instead.
"#.to_string()),
            context: None,
        });
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
        return HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
            done: false,
            reason: Some(
                r#"Successfully voted for this server!

Vote reminders for servers is <em>not</em> currently supported
"#
                .to_string(),
            ),
            context: None,
        });
    }
    error!("Vote Server Auth error");
    models::CustomError::ForbiddenGeneric.error_response()
}

/// Mini Index: Get Tags And Features
#[get("/mini-index")]
async fn mini_index(req: HttpRequest) -> Json<models::Index> {
    let mut mini_index = models::Index::new();

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    mini_index.tags = data.database.bot_list_tags().await;
    mini_index.features = data.database.bot_features().await;

    Json(mini_index)
}

/// User: Get Bot Settings
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
                HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                    done: false,
                    reason: Some("You are not allowed to edit this bot!".to_string()),
                    context: None,
                })
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
        models::CustomError::ForbiddenGeneric.error_response()
    }
}

/// Bot: Post Stats
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
        let resp = data.database.post_stats(bot_id, stats.into_inner(), &data.config.secrets.japi_key).await;
        match resp {
            Ok(()) => HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
                done: true,
                reason: Some("Successfully posted stats to v3 :)".to_string()),
                context: None,
            }),
            Err(err) => {
                HttpResponse::build(http::StatusCode::BAD_REQUEST).json(models::APIResponse {
                    done: false,
                    reason: Some(err.to_string()),
                    context: None,
                })
            }
        }
    } else {
        error!("Stat post auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}
