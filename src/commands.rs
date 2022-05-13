// Add, remove and delete commands from bots
use crate::models;
use actix_web::http::header::HeaderValue;
use actix_web::{delete, post, http, web, HttpRequest, HttpResponse};
use log::error;

#[post("/bots/{id}/commands")]
async fn add_command(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    res: web::Json<models::BotCommandVec>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let id = id.id.clone();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_bot(id, auth).await {
        for command in &res.commands {
            if command.name.is_empty() {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some(
                        "Command name must be at least 1 character long: ".to_string()
                            + &command.name,
                    ),
                    context: None,
                });
            }
            if command.description.is_empty() {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some(
                        "Command name must be at least 1 character long: ".to_string()
                            + &command.name,
                    ),
                    context: None,
                });
            }
            let ret = data.database.add_command(id, command).await;
            if ret.is_err() {
                return HttpResponse::BadRequest().json(models::APIResponse {
                    done: false,
                    reason: Some(ret.unwrap_err().to_string()),
                    context: None,
                });
            }
        }
        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    error!("Command post auth error");
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}

#[delete("/bots/{id}/commands")]
async fn delete_commands(
    req: HttpRequest,
    id: web::Path<models::FetchBotPath>,
    query: web::Query<models::CommandDeleteQuery>,
) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let id = id.id.clone();

    // Check auth
    let auth_default = &HeaderValue::from_str("").unwrap();
    let auth = req
        .headers()
        .get("Authorization")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();
    if data.database.authorize_bot(id, auth).await {
        // If nuke, delete all commands
        if query.nuke.unwrap_or(false) == true {
            data.database.delete_all_commands(id).await;
        }

        // If names, delete each command by name,
        if query.names.is_some() {
            let names = query.names.as_ref().unwrap();
            for cmd in names.split("|").collect::<Vec<&str>>() {
                data.database.delete_commands_by_name(id, cmd).await;
            }
        }

        // If ids, delete each command by id
        if query.ids.is_some() {
            let ids = query.ids.as_ref().unwrap();
            for cmd in ids.split("|").collect::<Vec<&str>>() {
                let id_parse = uuid::Uuid::parse_str(&cmd);
                if id_parse.is_err() {
                    continue;
                }
                let cmd_id = id_parse.unwrap();
                data.database.delete_commands_by_id(id, cmd_id).await;
            }
        }

        return HttpResponse::Ok().json(models::APIResponse::ok());
    }
    error!("Command delete auth error");
    HttpResponse::build(http::StatusCode::FORBIDDEN).json(models::APIResponse::err_small(&models::GenericError::Forbidden))
}
