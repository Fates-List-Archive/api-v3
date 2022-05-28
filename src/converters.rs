// Handle simple data conversions and webhook sending
use crate::models;
use actix_web::http::StatusCode;
use log::{debug, error};
use pulldown_cmark::{html::push_html, Options, Parser};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde_json::json;
use std::sync::Arc;
use ring::hmac;

pub fn invite_link(client_id: &str, invite: &str) -> String {
    if invite.starts_with("P:") && invite.len() > 2 {
        let inv_split = invite.split(':');
        let inv_vec = inv_split.collect::<Vec<&str>>();

        return format!(
            "https://discord.com/api/oauth2/authorize?client_id={bot_id}&permissions={perm}&scope=bot%20applications.commands",
            bot_id = client_id,
            perm = inv_vec.get(1).unwrap_or(&"0"),
        );
    } else if invite.is_empty() {
        return format!(
            "https://discord.com/api/oauth2/authorize?client_id={bot_id}&permissions={perm}&scope=bot%20applications.commands",
            bot_id = client_id,
            perm = 0,
        );
    }
    invite.to_string()
}

pub fn sanitize_description(
    long_desc_type: models::LongDescriptionType,
    description: &str,
) -> String {
    debug!("Sanitizing description");
    let mut html = String::new();
    if long_desc_type == models::LongDescriptionType::MarkdownServerSide {
        let options = Options::all();
        let md_parse = Parser::new_ext(description.as_ref(), options);
        push_html(&mut html, md_parse);
    } else {
        html = description.to_string();
    }

    ammonia::Builder::new()
        .rm_clean_content_tags(&["style", "iframe"])
        .add_tags(&[
            "span", "img", "video", "iframe", "style", "p", "br", "center", "div", "h1", "h2",
            "h3", "h4", "h5", "section", "article", "fl-lang",
        ])
        .add_generic_attributes(&[
            "id",
            "class",
            "style",
            "data-src",
            "data-background-image",
            "data-background-image-set",
            "data-background-delimiter",
            "data-icon",
            "data-inline",
            "data-height",
            "code",
        ])
        .add_tag_attributes("iframe", &["src", "height", "width"])
        .add_tag_attributes(
            "img",
            &[
                "src",
                "alt",
                "width",
                "height",
                "crossorigin",
                "referrerpolicy",
                "sizes",
                "srcset",
            ],
        )
        .clean(&html)
        .to_string()
}

pub fn create_token(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn flags_check(flag_list: &[i32], flag_vec: Vec<i32>) -> bool {
    for flag in flag_vec {
        if flag_list.contains(&flag) {
            return true;
        }
    }
    false
}

// Moved here due to 'static requirement
pub async fn send_discord_integration(
    client: Arc<serenity::http::client::Http>,
    webhook: String,
    vote_event: models::VoteWebhookEvent,
) {
    // Extract token from webhook
    let url = webhook.parse();
    if url.is_err() {
        error!("Webhook is not a valid URL!");
        return;
    }
    let url = url.unwrap();
    let parsed = serenity::utils::parse_webhook(&url);
    if parsed.is_none() {
        error!("Failed to parse webhook");
        return;
    }
    let (id, token) = parsed.unwrap();

    let vote = serenity::model::channel::Embed::fake(|e| {
        e.title("New Vote!")
            .description(format!("<@{}> has voted for your bot! You now have {} votes. **GG**", vote_event.user, vote_event.votes))
            .colour(serenity::utils::Colour::from_rgb(17, 247, 79))
    });    

    let mut map = serde_json::Map::new();

    map.insert("embeds".to_string(), json!(vec![vote]));

    let err = client.execute_webhook(id, token, true, &map).await;

    if err.is_err() {
        error!("Failed to send webhook: {:?}", err);
    }
}

// Moved here due to 'static requirement
pub async fn send_vote_webhook(
    requests: reqwest::Client,
    webhook: String,
    webhook_token: String,
    webhook_hmac_only: bool,
    vote_event: models::VoteWebhookEvent,
) {
    let mut tries = 0;
    while tries < 5 {
        let mut req = requests.post(webhook.as_str());

        if !webhook_hmac_only {
            req = req.header("Authorization", &webhook_token);
        }

        let hmac_data = serde_json::to_string(&vote_event);

        if hmac_data.is_err() {
            error!("Failed to serialize vote webhook data");
            return;
        }

        let hmac_data = hmac_data.unwrap();

        // Add HMAC
	let key = hmac::Key::new(hmac::HMAC_SHA512, webhook_token.as_bytes());

	let tag = hmac::sign(&key, hmac_data.as_bytes());

        let hmac = hex::encode(tag.as_ref());

        req = req.header("X-Webhook-Signature", hmac);

        let res = req.json(&vote_event).send().await;

        if res.is_err() {
            error!("Failed to send webhook: {}", res.unwrap_err());
            tries += 1;
            continue;
        }
        let res = res.unwrap();
        let status = res.status();
        if !(status.is_success() || status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN) {
            error!("Failed to send webhook: {}", res.text().await.unwrap());
            tries += 1;
            continue;
        } 
        debug!("Sent webhook with status code: {}", status);
        break;
    }
}
