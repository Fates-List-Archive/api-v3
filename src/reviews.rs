/// Handles reviews
use actix_web::{http, HttpRequest, get, post, web, HttpResponse, ResponseError, web::Json};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;
use serenity::model::prelude::*;

#[get("/reviews/{id}")]
async fn get_reviews(req: HttpRequest, info: web::Path<models::FetchBotPath>, query: web::Query<models::ReviewQuery>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let mut page = 1;
    let page_opt = query.page;

    if page_opt.is_some() {
        page = page_opt.unwrap();
    }

    if page < 1 {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Page must be greater than 0".to_string()),
            context: None,
        });
    }

    let per_page = 9;
    let offset = ((page as i64) - 1)*per_page;

    let reviews = data.database.get_reviews(info.id, query.target_type, per_page, offset).await;

    let mut parsed_review = models::ParsedReview {
        reviews,
        per_page,
        from: offset,
        stats: data.database.get_review_stats(info.id, query.target_type).await,
        user_review: None,
    };

    if let Some(user_id) = query.user_id {
        if let Some(user_review) = data.database.get_reviews_for_user(user_id, info.id, query.target_type).await {
            parsed_review.user_review = Some(user_review);
        }
    }

    return HttpResponse::Ok().json(parsed_review);
}

/// Page is there are it is needed for the future
#[post("/reviews/{id}")]
async fn add_review(req: HttpRequest, info: web::Path<models::FetchBotPath>, query: web::Query<models::ReviewQuery>, review: web::Json<models::Review>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let user_id = query.user_id;

    if user_id.is_none() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("User ID must be an i64".to_string()),
            context: None,
        });
    }

    let user_id = user_id.unwrap();

    let existing = data.database.get_reviews_for_user(user_id, info.id, query.target_type).await;

    if existing.is_some() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("You have already made a review for this bot. Please edit that instead".to_string()),
            context: None,
        });
    }

    return HttpResponse::BadRequest().json(models::APIResponse {
        done: false,
        reason: Some("Creation of reviews is currently not allowed due to ongoing maintenance".to_string()),
        context: None,
    });
}