extern crate env_logger;

use actix_web::{get, middleware, web, http, HttpResponse, App, HttpServer, HttpRequest, error::ResponseError, Responder};
use actix_web::middleware::Logger;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
extern crate inflector;
use log::{debug, error, info};
use actix_cors::Cors;
use thiserror::Error;

use actix_rt;

mod ipc;
mod models;
mod database;

#[derive(Deserialize, Serialize)]
struct APIResponse {
    done: bool,
    reason: Option<String>,
    error: Option<String>, // This is the error itself
}

#[derive(Error, Debug)]
enum CustomError {
    #[error("Not Found")]
    NotFoundGeneric,
    #[error("Forbidden")]
    ForbiddenGeneric,
    #[error("Unknown Internal Error")]
    Unknown
}

impl CustomError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFoundGeneric => "Not Found".to_string(),
            Self::ForbiddenGeneric => "Forbidden".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> http::StatusCode {
        match *self {
            Self::NotFoundGeneric  => http::StatusCode::NOT_FOUND,
            Self::ForbiddenGeneric => http::StatusCode::FORBIDDEN,
            Self::Unknown => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = APIResponse {
            reason: Some(self.to_string()),
            error: Some(self.name()),
            done: status_code.is_success(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}

struct AppState {
    database: database::Database,
}


#[get("/index")]
async fn index(req: HttpRequest, info: web::Query<models::IndexQuery>) -> impl Responder {
    let mut index = models::Index {
        top_voted: Vec::new(),
        certified: Vec::new(),
        tags: Vec::new(),
        features: HashMap::new(),
    };

    let data: &AppState = req.app_data::<web::Data<AppState>>().unwrap();

    if info.target_type.as_ref().unwrap_or(&"bot".to_string()) == "bot" {
        index.top_voted = data.database.index_bots(models::State::Approved).await;
        index.certified = data.database.index_bots(models::State::Certified).await;
        index.tags = data.database.bot_list_tags().await;
        ( 
            web::Json(index),
            http::StatusCode::OK,
        )
    } else {
        index.top_voted = data.database.index_servers(models::State::Approved).await;
        index.certified = data.database.index_servers(models::State::Certified).await;
        index.tags = data.database.server_list_tags().await;
        (
            web::Json(index),
            http::StatusCode::OK,
        )
    }
}

#[get("/code/{vanity}")]
async fn get_vanity(req: HttpRequest, code: web::Path<String>) -> HttpResponse {
    let data: &AppState = req.app_data::<web::Data<AppState>>().unwrap();
    let resolved_vanity = data.database.resolve_vanity(&code.into_inner()).await;
    match resolved_vanity {
        Some(data) => {
            return HttpResponse::build(http::StatusCode::OK).json(data);
        }
        _ => {
            let error = CustomError::NotFoundGeneric;
            return error.error_response();
        }
    }
}


async fn not_found(_req: HttpRequest) -> HttpResponse {
    CustomError::NotFoundGeneric.error_response()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "fates=debug,actix_web=info");
    env_logger::init();
    info!("Starting up...");
    let pool = database::Database::new(7, "postgres://localhost/fateslist", "redis://127.0.0.1:1001/1").await;
    
    debug!("Connected to postgres/redis");

    let app_state = web::Data::new(AppState {
        database: pool,
    });

    debug!("Connected to redis");
    
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
            .service(get_vanity)
    })
    .workers(6)
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
