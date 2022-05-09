/// Handles bot actions (view)

use crate::models;
use std::sync::Arc;
use uuid::Uuid;
use actix_web::http::header::HeaderValue;
use actix_web::{get, web, http, web::Json, HttpRequest, HttpResponse};
use log::{error, debug};

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

    // Check server cache
    let cache = data.database.server_cache.get(&id.id);
    
    match cache {
        Some(server) => {
            debug!("Server cache hit for {}", id.id);
            let mut server_clone = (*server).clone();
            server_clone.invite_link = invite_link;
            HttpResponse::Ok().json(server_clone)
        },
        None => {
            debug!("Server cache miss for {}", id.id);
            let server = data.database.get_server(id.id).await;
            match server {
                Some(mut server_data) => {
                    data.database.server_cache.insert(id.id, Arc::new(server_data.clone())).await;
                    // After inserting server into cache, then add invite link
                    server_data.invite_link = invite_link;
                    HttpResponse::Ok().json(server_data)
                },
                _ => HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err(&models::GenericError::NotFound)),
            }
        }
    }
}

// Get Random Server
#[get("/random-server")]
async fn random_server(req: HttpRequest) -> Json<models::IndexBot> {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let server = data.database.random_server().await;
    Json(server)
}