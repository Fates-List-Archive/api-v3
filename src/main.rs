#![allow(clippy::cast_sign_loss)]
#![allow(clippy::let_unit_value)]

use actix_cors::Cors;
use actix_web::dev::Service;
use actix_web::http::uri::PathAndQuery;
use actix_web::http::Uri;
use actix_web::middleware::Logger;
use actix_web::{http, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use bytes::Bytes;
use futures::future::FutureExt;
use log::{debug, error, info};
use std::sync::Arc;

mod appeal;
mod botactions;
mod serveractions;
mod commands;
mod converters;
mod core;
mod database;
mod docs;
mod login;
mod models;
mod packs;
mod reviews;
mod security;
mod stats;
mod user;
mod ws;
mod votes;
mod notifs;

use crate::models::APIResponse;

async fn not_found(_req: HttpRequest) -> HttpResponse {
    HttpResponse::build(http::StatusCode::NOT_FOUND).json(models::APIResponse::err_small(&models::GenericError::NotFound))
}

fn actix_handle_err<T: std::error::Error + 'static>(err: T) -> actix_web::error::Error {
    let response = HttpResponse::BadRequest().json(APIResponse::err(&err));
    actix_web::error::InternalError::from_response(err, response).into()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "fates=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");

    /* We have to create a new AppConfig to get a discord_http client
    This is also negligible cost
    */

    let app_config = models::AppConfig::default();

    let discord_main = app_config.discord_http;
    let discord_server = app_config.discord_http_server;

    let pool = database::Database::new(
        7,
        "postgres://localhost/fateslist",
        "redis://127.0.0.1:1001/1",
        /* Arc is used here for discord to provide shared ownership which is needed by discord integration support
        Cost for Arc is negligible here
        */
        Arc::new(discord_main),
        Arc::new(discord_server)
    )
    .await;

    debug!("Connected to postgres/redis");

    let client = reqwest::Client::builder()
        .user_agent("DiscordBot (https://fateslist.xyz, 0.1) FatesList-Lightleap-WarriorCats")
        .build()
        .unwrap();

    let app_state = web::Data::new(models::AppState {
        database: pool,
        config: models::AppConfig::default(),
        requests: client,
    });

    docs::document_routes();
    docs::document_enums();

    error!("This is a error");

    debug!("Connected to redis");

    debug!("Server is starting...");
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| !origin.as_bytes().ends_with(b"malware_domain_here"))
            .allowed_methods(vec![
                "GET", "HEAD", "PUT", "POST", "PATCH", "DELETE", "OPTIONS",
            ])
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
                http::header::HeaderName::from_bytes(b"Method").unwrap(),
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
            .wrap(middleware::NormalizePath::new(
                middleware::TrailingSlash::MergeOnly,
            ))
            .wrap_fn(|mut req, srv| {
                // Adapted from https://actix.rs/actix-web/src/actix_web/middleware/normalize.rs.html#89
                let head = req.head_mut();

                let original_path = head.uri.path();
                let path = original_path.replacen("/api/v2/", "/", 1);

                let mut parts = head.uri.clone().into_parts();
                let query = parts
                    .path_and_query
                    .as_ref()
                    .and_then(actix_web::http::uri::PathAndQuery::query);

                let path = match query {
                    Some(q) => Bytes::from(format!("{}?{}", path, q)),
                    None => Bytes::copy_from_slice(path.as_bytes()),
                };
                parts.path_and_query = Some(PathAndQuery::from_maybe_shared(path).unwrap());

                let uri = Uri::from_parts(parts).unwrap();
                req.match_info_mut().get_mut().update(&uri);
                req.head_mut().uri = uri;

                srv.call(req).map(|res| res)
            })
            .default_service(web::route().to(not_found))
            // Core
            .service(core::index)
            .service(core::ping)
            .service(core::mini_index) // Used Add Bot
            .service(core::resolve_vanity)
            .service(core::get_experiment_list)
            .service(core::get_partners)
            .service(core::search_list)
            .service(core::search_tags)
            .service(core::set_server_listing_by_web)

            // Votes
            .service(votes::create_bot_vote)
            .service(votes::create_server_vote)
            .service(votes::get_bot_votes)
            .service(votes::get_server_votes)

            // Login
            .service(login::get_oauth2)
            .service(login::do_oauth2)
            .service(login::get_frostpaw_client)
            .service(login::refresh_access_token)
            
            // Security
            .service(security::new_bot_token)
            .service(security::new_user_token)
            .service(security::new_server_token)
            .service(security::revoke_frostpaw_client_auth)
            
            // Bot Actions
            .service(botactions::add_bot)
            .service(botactions::edit_bot)
            .service(botactions::transfer_ownership)
            .service(botactions::delete_bot)
            .service(botactions::import_bot)
            .service(botactions::get_import_sources)
            .service(botactions::get_bot)
            .service(botactions::random_bot)
            .service(botactions::post_stats)            
            .service(botactions::get_bot_settings)


            // Server Actions
            .service(serveractions::get_server)
            .service(serveractions::random_server)

            // Appeal
            .service(appeal::appeal_bot)
            .service(appeal::appeal_server)

            // Packs
            .service(packs::add_pack)
            .service(packs::edit_pack)
            .service(packs::delete_pack)

            // User
            .service(user::get_user_from_id)
            .service(user::get_user_perms)
            .service(user::get_profile)
            .service(user::update_profile)
            .service(user::receive_profile_roles)

            // Review
            .service(reviews::get_reviews)
            .service(reviews::add_review)
            .service(reviews::edit_review)
            .service(reviews::delete_review)
            .service(reviews::vote_review)

            // Stats
            .service(stats::get_botlist_stats)

            // Commands
            .service(commands::add_command)
            .service(commands::delete_commands)

            // WS
            .service(ws::preview_description)
            .service(ws::bot_ws)

            // Notifications
            .service(notifs::get_notif_info)
            .service(notifs::subscribe)
            .service(notifs::test_notifs)
    })
    .workers(8)
    .bind("localhost:3010")?
    .run()
    .await
}
