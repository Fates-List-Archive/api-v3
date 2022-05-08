// A core endpoint is one that is absolutely essential for proper list functions
use crate::models;
use actix_web::{get, http, web, web::Json, HttpRequest, HttpResponse, ResponseError};
use strum::IntoEnumIterator;
use std::sync::Arc;

#[get("/index")]
async fn index(req: HttpRequest, info: web::Query<models::IndexQuery>) -> HttpResponse {
    let mut index = models::Index::new();

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let cache = data.database.index_cache.get(&info.target_type);
        
    if cache.is_some() {
        return HttpResponse::Ok().json(cache.unwrap());
    }

    let index = Arc::new(if info.target_type == models::TargetType::Bot {
        index.top_voted = data.database.index_bots(models::State::Approved).await;
        index.certified = data.database.index_bots(models::State::Certified).await;
        index.tags = data.database.bot_list_tags().await;
        index.new = data.database.index_new_bots().await;
        index.features = data.database.bot_features().await;

        index
    } else {
        index.top_voted = data.database.index_servers(models::State::Approved).await;
        index.certified = data.database.index_servers(models::State::Certified).await;
        index.new = data.database.index_new_servers().await;
        index.tags = data.database.server_list_tags().await;

        index 
   });
    data.database.index_cache.insert(info.target_type, index.clone()).await;
    return HttpResponse::Ok().json(index);
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

// Partners
#[get("/partners")]
async fn partners(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    HttpResponse::build(http::StatusCode::OK).json(&data.config.partners)
}

/// Search route.
#[get("/search")]
async fn search(req: HttpRequest, info: web::Query<models::SearchQuery>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let search = info.into_inner();

    let search_key = format!("{query}-{gc_from}-{gc_to}", query = search.q, gc_from = search.gc_from, gc_to = search.gc_to);

    let cached_resp = data.database.search_cache.get(&search_key);
    match cached_resp {
        Some(resp) => HttpResponse::Ok().json(resp),
        None => {
            let search_resp = Arc::new(data.database.search(search).await);
            data.database.search_cache.insert(search_key, search_resp.clone()).await;
            HttpResponse::Ok().json(search_resp)
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

/// Mini Index: Get Tags And Features
#[get("/mini-index")]
async fn mini_index(req: HttpRequest) -> Json<models::Index> {
    let mut mini_index = models::Index::new();

    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    mini_index.tags = data.database.bot_list_tags().await;
    mini_index.features = data.database.bot_features().await;

    Json(mini_index)
}
