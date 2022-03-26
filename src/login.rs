// Endpoints to handle login/logout
use crate::models;
use actix_web::cookie::time::Duration as CookieDuration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::http::header::HeaderValue;
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use log::error;
use std::collections::HashMap;
use std::time::Duration;
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

/// Creates a oauth2 login
#[post("/oauth2")]
async fn do_oauth2(req: HttpRequest, info: web::Json<models::OauthDoQuery>) -> HttpResponse {
    // Get code
    let code = info.code.clone();
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let auth_default = &HeaderValue::from_str("").unwrap();

    let redirect_url_domain = req
        .headers()
        .get("Frostpaw-Server")
        .unwrap_or(auth_default)
        .to_str()
        .unwrap();

    let redirect_uri = format!("{}/frostpaw/login", redirect_url_domain);

    let login = login_user(data, code, redirect_uri).await;

    match login {
        Err(err) => {
            error!("{:?}", err.to_string());
            HttpResponse::BadRequest().json(models::APIResponse {
                done: false,
                reason: Some(err.to_string()),
                context: None,
            })
        }
        Ok(user) => {
            let cookie_val = base64::encode(serde_json::to_string(&user).unwrap());
            let sunbeam_cookie = Cookie::build("sunbeam-session:warriorcats", cookie_val)
                .path("/")
                .domain("fateslist.xyz")
                .secure(true)
                .http_only(true)
                .max_age(CookieDuration::new(60 * 60 * 8, 0))
                .same_site(SameSite::Strict)
                .finish();
            return HttpResponse::Ok().cookie(sunbeam_cookie).json(user);
        }
    }
}

/// 'Deletes' (logs out) a oauth2 login
#[delete("/oauth2")]
async fn del_oauth2(req: HttpRequest) -> HttpResponse {
    let sunbeam_cookie = Cookie::build("sunbeam-session:warriorcats", "")
        .path("/")
        .domain("fateslist.xyz")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish();

    let mut resp = HttpResponse::Ok().json(models::APIResponse {
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

/// Actual Oauth2 impl
async fn login_user(
    data: &models::AppState,
    code: String,
    redirect_url: String,
) -> Result<models::OauthUserLogin, models::OauthError> {
    let mut params = HashMap::new();
    params.insert("client_id", data.config.secrets.client_id.clone());
    params.insert("client_secret", data.config.secrets.client_secret.clone());
    params.insert("grant_type", "authorization_code".to_string());
    params.insert("code", code);
    params.insert("redirect_uri", redirect_url);

    let access_token_exchange = data
        .requests
        .post("https://discord.com/api/v10/oauth2/token")
        .timeout(Duration::from_secs(10))
        .form(&params)
        .send()
        .await
        .map_err(models::OauthError::BadExchange)?;

    if !access_token_exchange.status().is_success() {
        let json = access_token_exchange
            .text()
            .await
            .map_err(models::OauthError::BadExchange)?;
        return Err(models::OauthError::BadExchangeJson(json));
    }

    let json = access_token_exchange
        .json::<models::OauthAccessTokenResponse>()
        .await
        .map_err(models::OauthError::BadExchange)?;

    let user_exchange = data
        .requests
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(json.access_token)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(models::OauthError::NoUser)?;

    if !user_exchange.status().is_success() {
        let json = user_exchange
            .text()
            .await
            .map_err(models::OauthError::BadExchange)?;
        return Err(models::OauthError::BadExchangeJson(json));
    }

    let json = user_exchange
        .json::<models::OauthUser>()
        .await
        .map_err(models::OauthError::NoUser)?;

    let data = data
        .database
        .create_user_oauth(json)
        .await
        .map_err(models::OauthError::SQLError)?;

    Ok(data)
}
