// A core endpoint is one that is absolutely essential
use actix_web::{http, HttpRequest, get, web, HttpResponse, ResponseError, web::Json};
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
    if code.starts_with("_") {
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

#[get("/_docs_template")]
async fn docs_tmpl(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).body(data.docs.clone())
}

// Bot route
#[get("/bots/{id}")]
async fn get_bot(req: HttpRequest, id: web::Path<models::FetchBotPath>, info: web::Query<models::FetchBotQuery>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let inner = info.into_inner();
    let id = id.into_inner();

    // This code *does not work IPC side yet due to needed flamepaw changes*
    if req.headers().contains_key("Frostpaw") {
        let auth_default = &HeaderValue::from_str("").unwrap();
        let auth = req.headers().get("Frostpaw-Auth").clone().unwrap_or(auth_default);
        let mut event_user: Option<String> = None;
        if !auth.clone().is_empty() {
            let auth_bytes = auth.to_str();
            match auth_bytes {
                Ok(auth_str) => {
                    let auth_split = auth_str.split("|");
                    let auth_vec = auth_split.collect::<Vec<&str>>();

                    let user_id = auth_vec.get(0).unwrap_or(&"");
                    let token = auth_vec.get(1).unwrap_or(&"");

                    let user_id_str = user_id.to_string();

                    let user_id_i64 = user_id_str.parse::<i64>().unwrap_or(0);

                    if data.database.authorize_user(user_id_i64, &token.to_string()).await {
                        event_user = Some(user_id_str);
                    }

                    let event = models::Event {
                        m: models::EventMeta {
                            e: models::EventName::BotView,
                            eid: Uuid::new_v4().to_hyphenated().to_string(),
                        },
                        ctx: models::EventContext {
                            target: id.id.to_string(),
                            target_type: models::EventTargetType::Bot,
                            user: event_user,
                        },
                        props: models::BotViewProp {
                            vote_page: req.headers().contains_key("Frostpaw-Vote-Page"),
                            widget: false,
                        }
                    }; 
                    data.database.ws_event(event).await;
                }
                Err(err) => {
                    error!("{}", err);
                }
            }
        }
    }


    let cached_bot = data.database.get_bot_from_cache(id.id).await;
    match cached_bot {
        Some(bot) => {
            HttpResponse::build(http::StatusCode::OK).json(bot)
        }
        None => {
            let bot = data.database.get_bot(id.id, inner.lang.unwrap_or_else(|| "en".to_string())).await;
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
async fn get_server(req: HttpRequest, id: web::Path<models::FetchBotPath>, info: web::Query<models::FetchBotQuery>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let inner = info.into_inner();
    let id = id.into_inner();

    // This code *does not work IPC side yet due to needed flamepaw changes*
    if req.headers().contains_key("Frostpaw") {
        let auth_default = &HeaderValue::from_str("").unwrap();
        let auth = req.headers().get("Frostpaw-Auth").clone().unwrap_or(auth_default);
        let mut event_user: Option<String> = None;
        if !auth.clone().is_empty() {
            let auth_bytes = auth.to_str();
            match auth_bytes {
                Ok(auth_str) => {
                    let auth_split = auth_str.split("|");
                    let auth_vec = auth_split.collect::<Vec<&str>>();

                    let user_id = auth_vec.get(0).unwrap_or(&"");
                    let token = auth_vec.get(1).unwrap_or(&"");

                    let user_id_str = user_id.to_string();

                    let user_id_i64 = user_id_str.parse::<i64>().unwrap_or(0);

                    if data.database.authorize_user(user_id_i64, &token.to_string()).await {
                        event_user = Some(user_id_str);
                    }

                    let event = models::Event {
                        m: models::EventMeta {
                            e: models::EventName::ServerView,
                            eid: Uuid::new_v4().to_hyphenated().to_string(),
                        },
                        ctx: models::EventContext {
                            target: id.id.to_string(),
                            target_type: models::EventTargetType::Server,
                            user: event_user,
                        },
                        props: models::BotViewProp {
                            vote_page: req.headers().contains_key("Frostpaw-Vote-Page"),
                            widget: false,
                        }
                    }; 
                    data.database.ws_event(event).await;
                }
                Err(err) => {
                    error!("{}", err);
                }
            }
        }
    }


    let cached_server = data.database.get_server_from_cache(id.id).await;
    match cached_server {
        Some(server) => {
            HttpResponse::build(http::StatusCode::OK).json(server)
        }
        None => {
            let server = data.database.get_server(id.id, inner.lang.unwrap_or_else(|| "en".to_string())).await;
            match server {
                Some(server_data) => {
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

    let query = info.q.clone().unwrap_or("fates".to_string());

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