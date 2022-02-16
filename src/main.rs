extern crate env_logger;

use actix_web::{get, middleware, web, http, App, HttpServer, HttpRequest, Result, Responder};
use actix_web::middleware::Logger;
use serde::{Deserialize, Serialize, Serializer};
use sqlx::postgres::PgPoolOptions;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use num_enum::TryFromPrimitive;
extern crate inflector;
use inflector::Inflector;
use log::{debug, error, log_enabled, info, Level};
use actix_cors::Cors;
extern crate redis;

mod ipc;
mod models;

#[derive(Deserialize, Serialize)]
#[derive(PartialEq)]
enum Status {
    Unknown = 0,
    Online = 1,
    Offline = 2, // Or invisible
    Idle = 3,
    DoNotDisturb = 4,
}


#[derive(Deserialize, Serialize)]
struct IndexBot {
    guild_count: i64,
    description: String,
    banner: Option<String>,
    nsfw: bool,
    votes: i64,
    state: models::State,
    user: models::User,
}

#[derive(Deserialize, Serialize)]
struct Tag {
    name: String,
    iconify_data: String,
    id: String,
    owner_guild: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Feature {
    name: String,
    viewed_as: String,
    description: String,
}

#[derive(Deserialize, Serialize)]
struct Index {
    top_voted: Vec<IndexBot>,
    certified: Vec<IndexBot>,
    tags: Vec<Tag>,
    features: HashMap<String, Feature>,
}

#[derive(Deserialize, Serialize)]
struct APIResponse {
    done: bool,
    reason: Option<String>,
}


#[derive(Deserialize)]
struct IndexQuery {
    target_type: Option<String>,
}

struct AppState {
    postgres: PgPool,
    redis: redis::Client,
}


#[get("/index")]
async fn index(req: HttpRequest, info: web::Query<IndexQuery>) -> impl Responder {
    let mut index = Index {
        top_voted: Vec::new(),
        certified: Vec::new(),
        tags: Vec::new(),
        features: HashMap::new(),
    };

    let data: &AppState = req.app_data::<web::Data<AppState>>().unwrap();

    if info.target_type.as_ref().unwrap_or(&"bot".to_string()) == "bot" {
        sqlx::query!("SELECT bot_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM bots WHERE state = 0 ORDER BY votes DESC LIMIT 12")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(async |row| {
                let bot = IndexBot {
                    guild_count: row.guild_count.unwrap_or(0),
                    description: row.description.clone().unwrap_or("No description set".to_string()),
                    banner: row.banner_card.clone(),
                    state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                    nsfw: row.nsfw.unwrap_or(false),
                    votes: row.votes.unwrap_or(0),
                    user: ipc::get_user(data.redis, row.bot_id).await,
                };
                index.top_voted.push(bot);
            });
        sqlx::query!("SELECT bot_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM bots WHERE state = 6 ORDER BY votes DESC LIMIT 12")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let bot = IndexBot {
                    guild_count: row.guild_count.unwrap_or(0),
                    description: row.description.clone().unwrap_or("No description set".to_string()),
                    banner: row.banner_card.clone(),
                    state: models::State::try_from(row.state).unwrap_or(models::State::Certified),
                    nsfw: row.nsfw.unwrap_or(false),
                    votes: row.votes.unwrap_or(0),
                    user: ipc::get_user(data.redis, row.bot_id).await,
                };
                index.certified.push(bot);
            });
        sqlx::query!("SELECT id, icon FROM bot_list_tags")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let tag = Tag {
                    name: row.id.to_title_case(),
                    iconify_data: row.icon.clone(),
                    id: row.id.clone(),
                    owner_guild: None,
                };
                index.tags.push(tag);
            });
        ( 
            web::Json(index),
            http::StatusCode::OK,
        )
    } else {
        sqlx::query!("SELECT guild_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM servers WHERE state = 0 ORDER BY votes DESC LIMIT 12")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let bot = IndexBot {
                    guild_count: row.guild_count.unwrap_or(0),
                    description: row.description.clone().unwrap_or("No description set".to_string()),
                    banner: row.banner_card.clone(),
                    state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                    nsfw: row.nsfw.unwrap_or(false),
                    votes: row.votes.unwrap_or(0),
                    user: ipc::get_user(data.redis, row.guild_id).await,
                };
                index.top_voted.push(bot);
            });
        sqlx::query!("SELECT guild_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM servers WHERE state = 6 ORDER BY votes DESC LIMIT 12")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let bot = IndexBot {
                    guild_count: row.guild_count.unwrap_or(0),
                    description: row.description.clone().unwrap_or("No description set".to_string()),
                    banner: row.banner_card.clone(),
                    state: models::State::try_from(row.state).unwrap_or(models::State::Certified),
                    nsfw: row.nsfw.unwrap_or(false),
                    votes: row.votes.unwrap_or(0),
                    user: ipc::get_user(data.redis, row.guild_id).await,
                };
                index.certified.push(bot);
            });
        sqlx::query!("SELECT id, name, iconify_data, owner_guild FROM server_tags")
            .fetch_all(&data.postgres)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let tag = Tag {
                    name: row.name.to_title_case(),
                    iconify_data: row.iconify_data.clone(),
                    id: row.id.clone(),
                    owner_guild: Some(row.owner_guild.to_string()),
                };
                index.tags.push(tag);
            });
        (
            web::Json(index),
            http::StatusCode::OK,
        )
    }
}

async fn not_found(_req: HttpRequest) -> impl Responder {
    let error = APIResponse {
        done: false,
        reason: Some("Not found".to_string()),
    };
    (
        web::Json(error),
        http::StatusCode::NOT_FOUND,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "fates=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://localhost/fateslist")
        .await
        .expect("Some error message");
    
    let client = redis::Client::open("redis://localhost:1001/1").unwrap();
    let app_state = web::Data::new(AppState {
        postgres: pool,
        redis: client,
    });
    debug!("Connected to postgres");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().ends_with(b"fateslist.xyz")
            })
            .allowed_methods(vec!["GET", "HEAD", "PUT", "POST", "PATCH", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION, 
                http::header::ACCEPT, 
                http::header::CONTENT_TYPE, 
                http::header::HeaderName::from_bytes(b"Frostpaw").unwrap(),
                http::header::HeaderName::from_bytes(b"Frostpaw-Auth").unwrap(),
                http::header::HeaderName::from_bytes(b"Frostpaw-Server").unwrap(),
                http::header::HeaderName::from_bytes(b"Frostpaw-Token").unwrap(),
                http::header::HeaderName::from_bytes(b"Frostpaw-Vote-Page").unwrap(),
                http::header::HeaderName::from_bytes(b"Method").unwrap()
            ])
            .supports_credentials()
            .max_age(3600);
        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .wrap(Logger::default())
            .default_service(web::route().to(not_found))
            .service(index)
    })
    .workers(6)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

