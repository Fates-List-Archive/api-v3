use sqlx::postgres::PgPoolOptions;
use sqlx::postgres::PgPool;
use crate::models;
use crate::ipc;
use crate::converters;
use deadpool_redis::{Config, Runtime};
use crate::inflector::Inflector;
use log::error;

pub struct Database {
    pool: PgPool,
    redis: deadpool_redis::Pool,
}

impl Database {
    pub async fn new(max_connections: u32, url: &str, redis_url: &str) -> Self {
        let cfg = Config::from_url(redis_url);
        Database {
            pool: PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(url)
                .await
                .expect("Could not initialize connection"),
            redis: cfg.create_pool(Some(Runtime::Tokio1)).unwrap(),
        }
    }
    pub async fn index_bots(&self, state: models::State) -> Vec<models::IndexBot> {
        let mut bots: Vec<models::IndexBot> = Vec::new();
        let rows = sqlx::query!(
            "SELECT bot_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM bots WHERE state = $1 ORDER BY votes DESC LIMIT 12",
            state as i32
        )
            .fetch_all(&self.pool)
            .await
            .unwrap();
        for row in rows.iter() {
            let bot = models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row.description.clone().unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| "https://api.fateslist.xyz/static/banner.webp".to_string()),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: ipc::get_user(self.redis.clone(), row.bot_id).await,
            };
            bots.push(bot);
        };
        bots
    }

    pub async fn bot_features(&self) -> Vec<models::Feature> {
        let mut features: Vec<models::Feature> = Vec::new();
        let rows = sqlx::query!(
            "SELECT id, name, viewed_as, description FROM features"
        )
            .fetch_all(&self.pool)
            .await
            .unwrap();
        for row in rows.iter() {
            let feature = models::Feature {
                id: row.id.clone(),
                name: row.name.clone(),
                viewed_as: row.viewed_as.clone(),
                description: row.description.clone(),
            };
            features.push(feature);
        };
        features
    }

    pub async fn index_new_bots(&self) -> Vec<models::IndexBot> {
        let mut bots: Vec<models::IndexBot> = Vec::new();
        let rows = sqlx::query!(
            "SELECT bot_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM bots WHERE state = $1 ORDER BY created_at DESC LIMIT 12",
            models::State::Approved as i32
        )
            .fetch_all(&self.pool)
            .await
            .unwrap();
        for row in rows.iter() {
            let bot = models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row.description.clone().unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| "https://api.fateslist.xyz/static/banner.webp".to_string()),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: ipc::get_user(self.redis.clone(), row.bot_id).await,
            };
            bots.push(bot);
        };
        bots
    }

    pub async fn get_server(&self, guild_id: i64) -> models::User {
        let row = sqlx::query!("SELECT guild_id::text AS id, name_cached AS username, avatar_cached AS avatar FROM servers WHERE guild_id = $1", guild_id)
            .fetch_one(&self.pool)
            .await
            .unwrap();
        models::User {
            id: row.id.unwrap(),
            username: row.username.clone(),
            disc: "0000".to_string(),
            avatar: row.avatar.unwrap_or_else(|| "".to_string()),
            bot: false,
        }
    }

    pub async fn index_servers(&self, state: models::State) -> Vec<models::IndexBot> {
        let mut servers: Vec<models::IndexBot> = Vec::new();
        let rows = sqlx::query!(
            "SELECT guild_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM servers WHERE state = $1 ORDER BY votes DESC LIMIT 12",
            state as i32
        )
            .fetch_all(&self.pool)
            .await
            .unwrap();
        for row in rows.iter() {
            let server = models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row.description.clone().unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| "https://api.fateslist.xyz/static/banner.webp".to_string()),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: self.get_server(row.guild_id).await,
            };
            servers.push(server);
        };
        servers
    }

    pub async fn index_new_servers(&self) -> Vec<models::IndexBot> {
        let mut servers: Vec<models::IndexBot> = Vec::new();
        let rows = sqlx::query!(
            "SELECT guild_id, flags, description, banner_card, state, votes, guild_count, nsfw FROM servers WHERE state = $1 ORDER BY created_at DESC LIMIT 12",
            models::State::Approved as i32
        )
            .fetch_all(&self.pool)
            .await
            .unwrap();
        for row in rows.iter() {
            let server = models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row.description.clone().unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| "https://api.fateslist.xyz/static/banner.webp".to_string()),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: self.get_server(row.guild_id).await,
            };
            servers.push(server);
        };
        servers
    }

    pub async fn bot_list_tags(&self) -> Vec<models::Tag> {
        let mut tags: Vec<models::Tag> = Vec::new();
        sqlx::query!("SELECT id, icon FROM bot_list_tags")
            .fetch_all(&self.pool)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let tag = models::Tag {
                    name: row.id.to_title_case(),
                    iconify_data: row.icon.clone(),
                    id: row.id.clone(),
                    owner_guild: None,
                };
                tags.push(tag);
            });
        tags
    }

    pub async fn server_list_tags(&self) -> Vec<models::Tag> {
        let mut tags: Vec<models::Tag> = Vec::new();
        sqlx::query!("SELECT id, name, iconify_data, owner_guild FROM server_tags")
            .fetch_all(&self.pool)
            .await
            .unwrap()
            .iter()
            .for_each(|row| {
                let tag = models::Tag {
                    name: row.name.to_title_case(),
                    iconify_data: row.iconify_data.clone(),
                    id: row.id.clone(),
                    owner_guild: Some(row.owner_guild.to_string()),
                };
                tags.push(tag);
            });
        tags
    }

    pub async fn resolve_vanity(&self, code: &str) -> Option<models::Vanity> {
        let row = sqlx::query!("SELECT type, redirect FROM vanity WHERE vanity_url = $1", code)
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(data) => {
                let target_type = match data.r#type {
                    Some(0) => {
                        "server"
                    },
                    Some(1) => {
                        "bot"
                    },
                    Some(2) => {
                        "profile"
                    },
                    _ => {
                        "bot"
                    },
                };
                let vanity = models::Vanity {
                    target_type: target_type.to_string(),
                    target_id: data.redirect.unwrap_or(0).to_string(),
                };
                Some(vanity)
            },
            Err(_) => {
                None
            }
        }
    }

    pub async fn get_vanity_from_id(&self, bot_id: i64) -> Option<String> {
        let row = sqlx::query!("SELECT vanity_url FROM vanity WHERE redirect = $1", bot_id)
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(data) => {
                data.vanity_url
            },
            Err(_) => {
                None
            }
        }
    }

    // Auth functions
    
    pub async fn authorize_user(&self, user_id: i64, token: &str) -> bool {
        let row = sqlx::query!(
            "SELECT COUNT(1) FROM users WHERE user_id = $1 AND api_token = $2",
            user_id,
            token,
        )
        .fetch_one(&self.pool)
        .await;
        row.is_ok()
    }
    pub async fn authorize_bot(&self, bot_id: i64, token: &str) -> bool {
        let row = sqlx::query!(
            "SELECT COUNT(1) FROM bots WHERE bot_id = $1 AND api_token = $2",
            bot_id,
            token,
        )
        .fetch_one(&self.pool)
        .await;
        row.is_ok()
    }

    // Get bot
    pub async fn get_bot(&self, bot_id: i64, lang: String) -> Option<models::Bot> {
        let row = sqlx::query!(
            "SELECT bot_id, created_at, last_stats_post, description, 
            css, flags, banner_card, banner_page, guild_count, shard_count, 
            shards, prefix, invite, invite_amount, features, bot_library 
            AS library, state, website, discord AS support, github, 
            user_count, votes, total_votes, donate, privacy_policy,
            nsfw, client_id, uptime_checks_total, uptime_checks_failed, 
            page_style, keep_banner_decor, long_description_type, 
            long_description FROM bots WHERE bot_id = $1 OR client_id = $1", 
            bot_id
        )
        .fetch_one(&self.pool) 
        .await;

        match row {
            Ok(data) => {
                // Handle client id
                let mut client_id: String = data.bot_id.to_string();
                match data.client_id {
                    Some(c_id) => {
                        client_id = c_id.to_string();
                    },
                    None => {
                    }
                };

                // Tags
                let tag_rows = sqlx::query!(
                    "SELECT tag FROM bot_tags WHERE bot_id = $1",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();

                let mut tags = Vec::new();

                for tag in tag_rows.iter() {
                    // Get tag info
                    let tag_info = sqlx::query!(
                        "SELECT icon FROM bot_list_tags WHERE id = $1",
                        tag.tag,
                    )
                    .fetch_one(&self.pool)
                    .await
                    .unwrap();
                    tags.push(models::Tag {
                        name: tag.tag.to_title_case(),
                        iconify_data: tag_info.icon.clone(),
                        id: tag.tag.to_string(),
                        owner_guild: None,
                    });
                }

                // Owners
                let owner_rows = sqlx::query!(
                    "SELECT owner, main FROM bot_owner WHERE bot_id = $1 ORDER BY main ASC",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();
                let mut owners = Vec::new();
                let mut owners_html = "".to_string();
                for row in owner_rows.iter() {
                    let user = ipc::get_user(self.redis.clone(), row.owner).await;
                    owners_html += &converters::owner_html(user.id.clone(), user.username.clone());
                    owners.push(models::BotOwner {
                        user: user.clone(),
                        main: row.main.unwrap_or(false),
                    });
                }

                // Action logs
                let mut action_logs = Vec::new();
                let action_log_rows = sqlx::query!(
                    "SELECT action, user_id, action_time, context FROM user_bot_logs WHERE bot_id = $1",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();

                for action_row in action_log_rows.iter() {
                    action_logs.push(models::ActionLog {
                        user_id: action_row.user_id.to_string(), 
                        action: action_row.action,
                        action_time: action_row.action_time, 
                        context: action_row.context.clone(),
                    })
                }

                // Make the struct
                let bot = models::Bot {
                    created_at: data.created_at,
                    last_stats_post: data.last_stats_post,
                    description: data.description.unwrap_or_else(|| "No description set".to_string()),
                    css: "<style>".to_string() + &data.css.unwrap_or_else(|| "".to_string()) + "</style>",
                    flags: data.flags.unwrap_or_default(),
                    banner_card: data.banner_card,
                    banner_page: data.banner_page,
                    guild_count: data.guild_count.unwrap_or(0),
                    shard_count: data.shard_count.unwrap_or(0),
                    shards: data.shards.unwrap_or_default(),
                    prefix: data.prefix,
                    invite: data.invite.clone(),
                    invite_link: converters::invite_link(client_id.clone(), data.invite.clone().unwrap_or_else(|| "".to_string())),
                    invite_amount: data.invite_amount.unwrap_or(0),
                    features: Vec::new(), // TODO
                    library: data.library.clone().unwrap_or_else(|| "".to_string()),
                    state: models::State::try_from(data.state).unwrap_or(models::State::Approved),
                    website: data.website,
                    support: data.support,
                    github: data.github,
                    user_count: data.user_count.unwrap_or(0),
                    votes: data.votes.unwrap_or(0),
                    total_votes: data.total_votes.unwrap_or(0),
                    donate: data.donate,
                    privacy_policy: data.privacy_policy,
                    nsfw: data.nsfw.unwrap_or(false),
                    keep_banner_decor: data.keep_banner_decor.unwrap_or(false),
                    client_id,
                    tags,
                    long_description: data.long_description.unwrap_or_else(|| "".to_string()),
                    owners,
                    vanity: self.get_vanity_from_id(bot_id).await.unwrap_or_else(|| "unknown".to_string()),
                    uptime_checks_total: data.uptime_checks_total,
                    uptime_checks_failed: data.uptime_checks_failed,
                    page_style: models::PageStyle::try_from(data.page_style).unwrap_or(models::PageStyle::Tabs),
                    long_description_type: models::LongDescriptionType::try_from(data.long_description_type).unwrap_or(models::LongDescriptionType::Html),
                    user: ipc::get_user(self.redis.clone(), data.bot_id).await,
                    owners_html,
                    action_logs,
                };
                Some(bot)
            }
            Err(err) => {
                error!("{}", err);
                None
            }  
        }
    }
}
