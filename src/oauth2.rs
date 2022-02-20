use crate::models;
use std::time::Duration;
use std::collections::HashMap;

pub async fn login_user(data: &models::AppState, code: String, redirect_url: String) -> Result<models::OauthUserLogin, models::OauthError> { 
    let mut params = HashMap::new();
    params.insert("client_id", data.config.secrets.client_id.clone());
    params.insert("client_secret", data.config.secrets.client_secret.clone());
    params.insert("grant_type", "authorization_code".to_string());
    params.insert("code", code);
    params.insert("redirect_uri", redirect_url);
    
    let access_token_exchange = data.requests.post("https://discord.com/api/v10/oauth2/token")
        .timeout(Duration::from_secs(10))
        .form(&params)
        .send()
        .await
        .map_err(models::OauthError::BadExchange)?;
            
    if !access_token_exchange.status().is_success() {
        let json = access_token_exchange.text().await.map_err(models::OauthError::BadExchange)?;
        return Err(models::OauthError::BadExchangeJson(json));
    }    
        
    let json = access_token_exchange.json::<models::OauthAccessTokenResponse>()
        .await
        .map_err(models::OauthError::BadExchange)?;
    
    let user_exchange = data.requests.get("https://discord.com/api/v10/users/@me")
        .bearer_auth(json.access_token)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(models::OauthError::NoUser)?;
            
    if !user_exchange.status().is_success() {
        let json = user_exchange.text().await.map_err(models::OauthError::BadExchange)?;
        return Err(models::OauthError::BadExchangeJson(json));
    }    
    
    let json = user_exchange.json::<models::OauthUser>()
        .await
        .map_err(models::OauthError::NoUser)?;
    
    let data = data.database.create_user_oauth(json).await.map_err(models::OauthError::SQLError)?;

    Ok(data)
}