/// Handles reviews
/// TODO, add websocket events *if desired*
use actix_web::{HttpRequest, get, post, patch, delete, web, HttpResponse, ResponseError};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;
use bigdecimal::FromPrimitive;

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

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if !data.database.authorize_user(user_id, auth).await {
        error!("Review Add Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    if review.parent_id.is_none() {
        let existing = data.database.get_reviews_for_user(user_id, info.id, query.target_type).await;

        if existing.is_some() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("You have already made a review for this bot. Please edit that instead".to_string()),
                context: None,
            });
        }
    } else {
        // Validate parent_id
        let parent_review = data.database.get_single_review(review.parent_id.unwrap()).await;
        if parent_review.is_none() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Parent review does not exist".to_string()),
                context: None,
            });
        }
    }

    if review.star_rating < bigdecimal::BigDecimal::from_i64(0).unwrap() || review.star_rating > bigdecimal::BigDecimal::from_i64(10).unwrap() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Star rating must be in range 1 to 10".to_string()),
            context: None,
        });    
    }

    if review.review_text.len() > 20000 || review.review_text.len() < 10 {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review text must be between 10 and 20000 characters".to_string()),
            context: None,
        });
    }

    if query.target_type == models::TargetType::Bot {
        let bot = data.database.get_bot(info.id).await;

        if bot.is_none() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Bot does not exist".to_string()),
                context: None,
            });
        }
    } else if query.target_type == models::TargetType::Server {
        let server = data.database.get_server(info.id).await;

        if server.is_none() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Server does not exist".to_string()),
                context: None,
            });
        }
    } else {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Target type must be either 'bot' or 'server'".to_string()),
            context: None,
        });
    }    

    let res = data.database.add_review(review.into_inner(), user_id, info.id, query.target_type).await;

    if res.is_err() {
        let err = res.err().unwrap().to_string();
        error!("Error adding review: {:?}", err);
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some(err),
            context: None,
        });
    }

    return HttpResponse::Ok().json(models::APIResponse {
        done: true,
        reason: Some("Successfully added review".to_string()),
        context: None,
    });
}

/// The FetchBotPath is not needed but we need to maintain a uniform API
#[patch("/reviews/{id}")]
async fn edit_review(req: HttpRequest, _: web::Path<models::FetchBotPath>, query: web::Query<models::ReviewQuery>, review: web::Json<models::Review>) -> HttpResponse {
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

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if !data.database.authorize_user(user_id, auth).await {
        error!("Review Add Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    if review.star_rating < bigdecimal::BigDecimal::from_i64(0).unwrap() || review.star_rating > bigdecimal::BigDecimal::from_i64(10).unwrap() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Star rating must be in range 1 to 10".to_string()),
            context: None,
        });    
    }

    if review.review_text.len() > 20000 || review.review_text.len() < 10 {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review text must be between 10 and 20000 characters".to_string()),
            context: None,
        });
    }

    // Check review id
    if review.id.is_none() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review ID must be an i64".to_string()),
            context: None,
        });
    }

    let review_id = review.id.unwrap();

    // Verify review ownership
    let review_orig = data.database.get_single_review(review_id).await;

    if review_orig.is_none() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review does not exist".to_string()),
            context: None,
        });
    }

    let review_orig = review_orig.unwrap();

    if review_orig.user.id != user_id.to_string() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("You do not own this review".to_string()),
            context: None,
        });
    }

    let res = data.database.edit_review(review.into_inner()).await;

    if res.is_err() {
        let err = res.err().unwrap().to_string();
        error!("Error adding review: {:?}", err);
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some(err),
            context: None,
        });
    }

    return HttpResponse::Ok().json(models::APIResponse {
        done: true,
        reason: Some("Successfully edited review".to_string()),
        context: None,
    });
}

#[delete("/reviews/{rid}")]
async fn delete_review(req: HttpRequest, info: web::Path<models::ReviewDeletePath>, query: web::Query<models::ReviewQuery>) -> HttpResponse {
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

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if !data.database.authorize_user(user_id, auth).await {
        error!("Review Add Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    let review_id = uuid::Uuid::parse_str(&info.rid);
    if review_id.is_err() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review ID must be a valid UUID".to_string()),
            context: None,
        });
    }
    let review_id = review_id.unwrap();

    // Verify review ownership
    let review_orig = data.database.get_single_review(review_id).await;

    if review_orig.is_none() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review does not exist".to_string()),
            context: None,
        });
    }

    let review_orig = review_orig.unwrap();

    if review_orig.user.id != user_id.to_string() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("You do not own this review".to_string()),
            context: None,
        });
    }

    let res = data.database.delete_review(review_id).await;

    if res.is_err() {
        let err = res.err().unwrap().to_string();
        error!("Error deleting review: {:?}", err);
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some(err),
            context: None,
        });
    }

    return HttpResponse::Ok().json(models::APIResponse {
        done: true,
        reason: Some("Successfully deleted review".to_string()),
        context: None,
    });
}


#[patch("/reviews/{rid}/votes")]
async fn vote_review(req: HttpRequest, info: web::Path<models::ReviewDeletePath>, vote: web::Json<models::ReviewVote>) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let user_id = vote.user_id.parse::<i64>();

    if user_id.is_err() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("User ID must be an i64".to_string()),
            context: None,
        });
    }

    let user_id = user_id.unwrap();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if !data.database.authorize_user(user_id, auth).await {
        error!("Review Vote Auth error");
        return models::CustomError::ForbiddenGeneric.error_response();
    }

    let review_id = uuid::Uuid::parse_str(&info.rid);
    if review_id.is_err() {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some("Review ID must be a valid UUID".to_string()),
            context: None,
        });
    }
    let review_id = review_id.unwrap();

    let upvote = vote.upvote;

    let res = data.database.get_review_votes(review_id).await;

    if (upvote && res.upvotes.contains(&vote.user_id)) || (!upvote && res.downvotes.contains(&vote.user_id)) {
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some(
                format!(
                    "You have already voted on this review. Click the {button} to {button} it.",
                    button = if upvote { "upvote" } else { "downvote" }
                )
            ),
            context: None,
        });
    }

    let res = data.database.add_review_vote(review_id, user_id, upvote).await;
    if res.is_err() {
        let err = res.err().unwrap().to_string();
        error!("Error adding review vote: {:?}", err);
        return HttpResponse::BadRequest().json(models::APIResponse {
            done: false,
            reason: Some(err),
            context: None,
        });
    }

    return HttpResponse::Ok().json(models::APIResponse {
        done: true,
        reason: Some("Successfully voted for this review".to_string()),
        context: None,
    });
}
