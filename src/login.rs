// Endpoints to handle login/logout
use actix_web::{http, HttpRequest, get, delete, web, HttpResponse, ResponseError, web::Json};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::http::header::HeaderValue;
use crate::models;
use log::error;
use uuid::Uuid;

/// Returns the oauth2 link to use for login
#[get("/oauth2")]
async fn get_oauth2(req: HttpRequest) -> HttpResponse {
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();
    let state = Uuid::new_v4().to_hyphenated().to_string();
    let auth_default = &HeaderValue::from_str("").unwrap();
    let url = format!(
        "https://discord.com/oauth2/authorize?client_id={discord_client_id}&redirect_uri={redirect_url_domain}/frostpaw/login&state={state}&scope=identify&response_type=code",
        discord_client_id = data.config.secrets.client_id,
        redirect_url_domain = req.headers().get("Frostpaw-Server").unwrap_or(auth_default).to_str().unwrap(),
        state = state,
    );
    HttpResponse::Ok().json(models::APIResponse {
        done: true,
        reason: None,
        context: Some(url),
    })
}

/// 'Deletes' (logs out) a oauth2 login
#[delete("/oauth2")]
async fn del_oauth2(req: HttpRequest) -> HttpResponse {
    let sunbeam_cookie = Cookie::build("sunbeam-session:warriorcats","")
    .path("/")
    .domain("fateslist.xyz")
    .secure(true)
    .http_only(true)
    .same_site(SameSite::Lax) 
    .finish();

    let mut resp = HttpResponse::Ok()
    .json(models::APIResponse {
        done: true,
        reason: None,
        context: None,
    });
    let cookie_del = resp.add_removal_cookie(&sunbeam_cookie);

    match cookie_del {
        Err(err) => {
            error!("{}", err);
        }
        _ => {}
    }

    resp
}