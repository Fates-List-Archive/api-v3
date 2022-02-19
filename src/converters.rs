// Handle simple data conversions
use crate::models;
use pulldown_cmark::{Parser, Options, html::push_html};
use log::debug;

pub fn invite_link(client_id: String, invite: String) -> String {
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
    } else {
        invite
    }
}

pub fn owner_html(id: String, username: String) -> String {
    return format!(
        "<a class='long-desc-link' href='/profile/{id}'>{username}</a><br/>",
        id = id,
        username = username,
    )
}

pub fn sanitize_description(long_desc_type: models::LongDescriptionType, description: String) -> String {
    // TODO: Check if stored in redis
    debug!("Sanitizing description");
    let mut html = String::new();
    if long_desc_type == models::LongDescriptionType::MarkdownServerSide {
        let options = Options::all();
        let md_parse = Parser::new_ext(description.as_ref(), options);
        push_html(&mut html, md_parse);
    } else {
        html = description.clone();
    }

    ammonia::Builder::new()
    .rm_clean_content_tags(&["style", "iframe"])
    .add_tags(&["span", "img", "video", "iframe", "style", "p", "br", "center", "div", "h1", "h2", "h3", "h4", "h5", "section", "article", "fl-lang"]) 
    .add_generic_attributes(&["id", "class", "style", "data-src", "data-background-image", "data-background-image-set", "data-background-delimiter", "data-icon", "data-inline", "data-height", "code"])
    .add_tag_attributes("iframe", &["src", "height", "width"])
    .add_tag_attributes("img", &["src", "alt", "width", "height", "crossorigin", "referrerpolicy", "sizes", "srcset"])
    .clean(&html)
    .to_string()
}