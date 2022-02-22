#![feature(derive_default_enum)]

extern crate env_logger;

use actix_web::{middleware, web, http, HttpResponse, App, HttpServer, HttpRequest, error::ResponseError};
use actix_web::middleware::Logger;
extern crate inflector;
use log::{debug, error, info};
use actix_cors::Cors;
use bytes::Bytes;
use actix_web::dev::Service;
use actix_web::http::Uri;
use actix_web::http::uri::PathAndQuery;
use futures::future::FutureExt;

mod ipc;
mod models;
mod database;
mod core;
mod login;
mod docs;
mod converters;
mod security;

use crate::models::APIResponse;

async fn not_found(_req: HttpRequest) -> HttpResponse {
    models::CustomError::NotFoundGeneric.error_response()
}

fn actix_handle_err<T: std::error::Error + 'static>(err: T) -> actix_web::error::Error {
    let response = HttpResponse::BadRequest().json(APIResponse {
        done: false,
        reason: Some(err.to_string()),
	context: None,
    });
    actix_web::error::InternalError::from_response(err, response).into()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "fates=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");
    let pool = database::Database::new(7, "postgres://localhost/fateslist", "redis://127.0.0.1:1001/1").await;
    
    debug!("Connected to postgres/redis");

    let client = reqwest::Client::builder()
    .user_agent("DiscordBot (https://fateslist.xyz, 0.1) FatesList-Lightleap-WarriorCats")
    .build()
    .unwrap();

    let app_state = web::Data::new(models::AppState {
        database: pool,
        docs: docs::document_routes(),
        config: models::AppConfig::default(),
        requests: client,
    });

    error!("This is a error");

    debug!("Connected to redis");
    
    debug!("Server is starting...");
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
                http::header::HeaderName::from_bytes(b"Frostpaw-Invite").unwrap(),
                http::header::HeaderName::from_bytes(b"Method").unwrap()
            ])
            .supports_credentials()
            .max_age(3600);
        App::new()
            .app_data(app_state.clone())
            .app_data(
                web::JsonConfig::default()
                    .limit(1024 * 1024 * 10)
                    .error_handler(|err, _req| actix_handle_err(err)),
            )
            .app_data(web::QueryConfig::default().error_handler(|err, _req| actix_handle_err(err)))
            .app_data(web::PathConfig::default().error_handler(|err, _req| actix_handle_err(err)))
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .wrap(Logger::default())
            .wrap(middleware::NormalizePath::new(middleware::TrailingSlash::MergeOnly))
            .wrap_fn(|mut req, srv| {
                // Adapted from https://actix.rs/actix-web/src/actix_web/middleware/normalize.rs.html#89
                let head = req.head_mut();

                let original_path = head.uri.path();
                let path = original_path.replacen("/api/v2/", "/", 1);

                let mut parts = head.uri.clone().into_parts();
                let query = parts.path_and_query.as_ref().and_then(|pq| pq.query());

                let path = match query {
                    Some(q) => Bytes::from(format!("{}?{}", path, q)),
                    None => Bytes::copy_from_slice(path.as_bytes()),
                };
                parts.path_and_query = Some(PathAndQuery::from_maybe_shared(path).unwrap());

                let uri = Uri::from_parts(parts).unwrap();
                req.match_info_mut().get_mut().update(&uri);
                req.head_mut().uri = uri;

                srv.call(req).map(|res| {
                    res
                })
            })    
            .default_service(web::route().to(not_found))
            .service(core::index)
            .service(core::mini_index) // Add Bot
            .service(core::get_vanity)
            .service(core::docs_tmpl)
            .service(core::policies)
            .service(core::partners)
            .service(core::get_bot)
            .service(core::get_server)
            .service(core::search)
            .service(core::random_bot)
            .service(core::random_server)
            .service(core::has_user_voted)
            .service(core::post_stats)
            .service(core::get_bot_settings)
            .service(login::get_oauth2)
            .service(login::del_oauth2)
            .service(login::do_oauth2)
            .service(security::new_bot_token)
            .service(security::new_user_token)
    })
    .workers(6)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
