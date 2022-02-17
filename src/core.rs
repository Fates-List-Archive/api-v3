
use actix_web::{http, HttpResponse, HttpRequest, error::ResponseError};
use paperclip::actix::{
    // extension trait for actix_web::App and proc-macro attributes
    OpenApiExt, api_v2_operation,
    // If you prefer the macro syntax for defining routes, import the paperclip macros
    // get, post, put, delete
    // use this instead of actix_web::web
    web, get
};
use crate::models;
use std::collections::HashMap;

#[api_v2_operation()]
#[get("/index")]
/// Returns the index page
async fn index(req: HttpRequest, info: web::Query<models::IndexQuery>) -> (web::Json<models::Index>, http::StatusCode) {
    let mut index = models::Index {
        top_voted: Vec::new(),
        certified: Vec::new(),
        new: Vec::new(),
        tags: Vec::new(),
        features: HashMap::new(),
    };

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    if info.target_type.as_ref().unwrap_or(&"bot".to_string()) == "bot" {
        index.top_voted = data.database.index_bots(models::State::Approved).await;
        index.certified = data.database.index_bots(models::State::Certified).await;
        index.tags = data.database.bot_list_tags().await;
        index.new = data.database.index_new_bots().await;
        ( 
            web::Json(index),
            http::StatusCode::OK,
        )
    } else {
        index.top_voted = data.database.index_servers(models::State::Approved).await;
        index.certified = data.database.index_servers(models::State::Certified).await;
        index.new = data.database.index_new_servers().await;
        index.tags = data.database.server_list_tags().await;
        (
            web::Json(index),
            http::StatusCode::OK,
        )
    }
}

#[get("/code/{vanity}")]
async fn get_vanity(req: HttpRequest, code: web::Path<String>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let resolved_vanity = data.database.resolve_vanity(&code.into_inner()).await;
    match resolved_vanity {
        Some(data) => {
            return HttpResponse::build(http::StatusCode::OK).json(data);
        }
        _ => {
            let error = models::CustomError::NotFoundGeneric;
            return error.error_response();
        }
    }
}

