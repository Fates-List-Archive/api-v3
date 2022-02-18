// Handle simple data conversions

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
        "<a class='long-desc-link' href='/profile/{id}'>{username}</a>",
        id = id,
        username = username,
    )
}