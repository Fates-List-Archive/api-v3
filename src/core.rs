
use actix_web::{http, HttpRequest, get, web, HttpResponse, ResponseError, web::Json};
use crate::models;

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
    let bot = data.database.get_bot(id.into_inner().id, inner.lang.unwrap_or_else(|| "en".to_string())).await;
    match bot {
        Some(bot_data) => {
            HttpResponse::build(http::StatusCode::OK).json(bot_data)
        }
        _ => {
            models::CustomError::NotFoundGeneric.error_response()
        }
    }
}
