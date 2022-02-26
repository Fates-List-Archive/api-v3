// A core endpoint is one that is absolutely essential for proper list functions
use actix_web::{http, HttpRequest, get, post, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;
use uuid::Uuid;


#[get("/index")]
async fn index(req: HttpRequest, info: web::Query<models::IndexQuery>) -> Json<models::Index> {
    let mut index = models::Index::new();

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    if info.target_type.as_ref().unwrap_or(&"bot".to_string()) == "bot" {
        index.top_voted = data.database.index_bots(models::State::Approved).await;
        index.certified = data.database.index_bots(models::State::Certified).await;
        index.tags = data.database.bot_list_tags().await;
        index.new = data.database.index_new_bots().await;
        index.features = data.database.bot_features().await;
    } else {
        index.top_voted = data.database.index_servers(models::State::Approved).await;
        index.certified = data.database.index_servers(models::State::Certified).await;
        index.new = data.database.index_new_servers().await;
        index.tags = data.database.server_list_tags().await;
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
        Some(data) => {
            HttpResponse::build(http::StatusCode::OK).json(data)
        }
        _ => {
            models::CustomError::NotFoundGeneric.error_response()
        }
    }
}

// Docs template (not yet documented)
#[get("/_docs_template")]
async fn docs_tmpl(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).body(data.docs.clone())
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

                    if data.database.authorize_user(user_id_i64, token.as_ref()).await {
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
            },
            props: models::BotViewProp {
                vote_page: req.headers().contains_key("Frostpaw-Vote-Page"),
                widget: false,
                invite: req.headers().contains_key("Frostpaw-Invite")
            }
        }; 
        data.database.ws_event(event).await;
    }

    if req.headers().contains_key("Frostpaw-Invite") {
        data.database.update_bot_invite_amount(id.id).await;
    }

    let cached_bot = data.database.get_bot_from_cache(id.id).await;
    match cached_bot {
        Some(bot) => {
            HttpResponse::build(http::StatusCode::OK).json(bot)
        }
        None => {
            let bot = data.database.get_bot(id.id).await;
            match bot {
                Some(bot_data) => {
                    HttpResponse::build(http::StatusCode::OK).json(bot_data)
                }
                _ => {
                    models::CustomError::NotFoundGeneric.error_response()
                }
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

                    if data.database.authorize_user(user_id_i64, token.as_ref()).await {
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
            },
            props: models::BotViewProp {
                vote_page: req.headers().contains_key("Frostpaw-Vote-Page"),
                widget: false,
                invite: req.headers().contains_key("Frostpaw-Invite")
            }
        }; 
        data.database.ws_event(event).await;
    }

    let mut invite_link: Option<String> = None;

    // Server invite handling using GUILDINVITE ipc
    if req.headers().contains_key("Frostpaw-Invite") {
        invite_link = Some(data.database.resolve_guild_invite(
            id.id, 
            event_user.unwrap_or_else(|| "0".to_string()).parse::<i64>().unwrap_or(0)
        ).await);
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
                _ => {
                    models::CustomError::NotFoundGeneric.error_response()
                }
            }
        }
    }
}

// Search route


#[get("/search")]
async fn search(req: HttpRequest, info: web::Query<models::SearchQuery>) -> Json<models::Search> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let query = info.q.clone().unwrap_or_else(|| "fates".to_string());

    let cached_resp = data.database.get_search_from_cache(query.clone()).await;
    match cached_resp {
        Some(resp) => {
            Json(resp)
        }
        None => {
            let search_resp = data.database.search(query).await;
            Json(search_resp)
        }
    }
}

// Search Tags
#[get("/search-tags")]
async fn search_tags(req: HttpRequest, info: web::Query<models::SearchQuery>) -> Json<models::Search> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let query = info.q.clone().unwrap_or_else(|| "music".to_string());
    let search_resp = data.database.search_tags(query).await;
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
async fn get_bot_settings(req: HttpRequest, info: web::Path<models::GetUserBotPath>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let user_id = info.user_id;

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if data.database.authorize_user(user_id, auth).await {
        let resp = data.database.get_bot_settings(info.bot_id).await;
        match resp {
            Ok(bot) => {
                // Check if in owners before returning
                for owner in &bot.owners {
                    let id = owner.user.id.parse::<i64>().unwrap_or(0);
                    if id == user_id {
                        return HttpResponse::build(http::StatusCode::OK).json(models::BotSettings {
                            bot,
                            context: models::BotSettingsContext {
                                tags: data.database.bot_list_tags().await,
                                features: data.database.bot_features().await,
                            }
                        });
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
async fn post_stats(req: HttpRequest, id: web::Path<models::FetchBotPath>, stats: web::Json<models::BotStats>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let bot_id = id.id.clone();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if data.database.authorize_bot(bot_id, auth).await {
        let resp = data.database.post_stats(bot_id, stats.into_inner()).await;
        match resp {
            Ok(()) => {
                HttpResponse::build(http::StatusCode::OK).json(models::APIResponse {
                    done: true,
                    reason: Some("Successfully posted stats to v3 :)".to_string()),
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
        error!("Stat post auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}