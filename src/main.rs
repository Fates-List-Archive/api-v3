#![feature(derive_default_enum)]

extern crate env_logger;

use actix_web::{middleware, web, http, HttpResponse, App, HttpServer, HttpRequest, error::ResponseError};
use actix_web::middleware::Logger;
extern crate inflector;
use log::{debug, error, info};
use actix_cors::Cors;

use actix_rt;

mod ipc;
mod models;
mod database;
mod core;
mod docs;

async fn not_found(_req: HttpRequest) -> HttpResponse {
    models::CustomError::NotFoundGeneric.error_response()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "fates=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");
    let pool = database::Database::new(7, "postgres://localhost/fateslist", "redis://127.0.0.1:1001/1").await;
    
    debug!("Connected to postgres/redis");

    let app_state = web::Data::new(models::AppState {
        database: pool,
        docs: docs::document_routes(),
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
            .service(core::index)
            .service(core::get_vanity)
            .service(core::docs_tmpl)
    })
    .workers(6)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
