// Add, remove and delete resources from bots/servers
use actix_web::{HttpRequest, post, delete, web, HttpResponse, ResponseError};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;

#[post("/resources/{id}")]
async fn add_resource(
    req: HttpRequest, 
    id: web::Path<models::FetchBotPath>, 
    target_type: web::Query<models::TargetQuery>, 
    res: web::Json<models::Resource>
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let id = id.id.clone();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if (
        target_type.target_type == models::TargetType::Bot && data.database.authorize_bot(id, auth).await
    ) || (
        target_type.target_type == models::TargetType::Server && data.database.authorize_server(id, auth).await
    ) {
        // Add resource
        let res = res.into_inner();

        if !res.resource_link.starts_with("https://") {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Resource link must start with https://".to_string()),
                context: Some("Check error".to_string())
            });        
        }

        if res.resource_description.len() < 5 {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Resource description must be at least 5 characters long".to_string()),
                context: Some("Check error".to_string())
            });        
        }

        if res.resource_title.len() < 5 {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Resource title must be at least 5 characters long".to_string()),
                context: Some("Check error".to_string())
            });        
        }

        let res = data.database.add_resource(id, target_type.target_type, res).await;
        if res.is_ok() {
            return HttpResponse::Ok().json(models::APIResponse {
                done: true,
                reason: Some("Successfully added resource to v3 :)".to_string()),
                context: None,
            });
        } else {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string())
            });        
        }
    } else {
        error!("Resource post auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}

#[delete("/resources/{id}")]
async fn delete_resource(
    req: HttpRequest, 
    id: web::Path<models::FetchBotPath>, 
    query: web::Query<models::ResourceDeleteQuery>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let id = id.id.clone();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req.headers().get("Authorization").unwrap_or(auth_default).to_str().unwrap();
    if (
        query.target_type == models::TargetType::Bot && data.database.authorize_bot(id, auth).await
    ) || (
        query.target_type == models::TargetType::Server && data.database.authorize_server(id, auth).await
    ) {
        // Get resource owner
        let resource_id = uuid::Uuid::parse_str(&query.id);
        if resource_id.is_err() {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Resource ID must be a valid UUID".to_string()),
                context: None,
            });
        }
        let resource_id = resource_id.unwrap();
    
        let resource_owned = data.database.resource_exists(resource_id, id, query.target_type).await;

        if !resource_owned {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some("Resource does not exist under this bot".to_string()),
                context: None,
            });
        }

        let res = data.database.delete_resource(resource_id).await;
        if res.is_ok() {
            return HttpResponse::Ok().json(models::APIResponse {
                done: true,
                reason: Some("Successfully added review to v3 :)".to_string()),
                context: None,
            });
        } else {
            return HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(res.unwrap_err().to_string()),
                context: Some("Check error".to_string())
            });        
        }
    } else {
        error!("Resource post auth error");
        models::CustomError::ForbiddenGeneric.error_response()
    }
}