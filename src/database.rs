use sqlx;
use sqlx::postgres::PgPoolOptions;
use sqlx::postgres::PgPool;
use crate::models;
use crate::ipc;
use deadpool_redis;
use deadpool_redis::{Config, Runtime};
use crate::inflector::Inflector;

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
    pub async fn index_bots(self: &Self, state: models::State) -> Vec<models::IndexBot> {
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
                description: row.description.clone().unwrap_or("No description set".to_string()),
                banner: row.banner_card.clone(),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: ipc::get_user(self.redis.clone(), row.bot_id).await,
            };
            bots.push(bot);
        };
        bots
    }

    pub async fn index_new_bots(self: &Self) -> Vec<models::IndexBot> {
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
                description: row.description.clone().unwrap_or("No description set".to_string()),
                banner: row.banner_card.clone(),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: ipc::get_user(self.redis.clone(), row.bot_id).await,
            };
            bots.push(bot);
        };
        bots
    }

    pub async fn get_server(self: &Self, guild_id: i64) -> models::User {
        let row = sqlx::query!("SELECT guild_id::text AS id, name_cached AS username, avatar_cached AS avatar FROM servers WHERE guild_id = $1", guild_id)
            .fetch_one(&self.pool)
            .await
            .unwrap();
        models::User {
            id: row.id.unwrap().clone(),
            username: row.username.clone(),
            disc: "0000".to_string(),
            avatar: row.avatar.unwrap_or("".to_string()).clone(),
            bot: false,
        }
    }

    pub async fn index_servers(self: &Self, state: models::State) -> Vec<models::IndexBot> {
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
                description: row.description.clone().unwrap_or("No description set".to_string()),
                banner: row.banner_card.clone(),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: self.get_server(row.guild_id).await,
            };
            servers.push(server);
        };
        servers
    }

    pub async fn index_new_servers(self: &Self) -> Vec<models::IndexBot> {
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
                description: row.description.clone().unwrap_or("No description set".to_string()),
                banner: row.banner_card.clone(),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                user: self.get_server(row.guild_id).await,
            };
            servers.push(server);
        };
        servers
    }

    pub async fn bot_list_tags(self: &Self) -> Vec<models::Tag> {
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

    pub async fn server_list_tags(self: &Self) -> Vec<models::Tag> {
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

    pub async fn resolve_vanity(self: &Self, code: &str) -> Option<models::Vanity> {
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
                return Some(vanity);
            },
            Err(_) => {
                return None;
            }
        }
    }

    // Auth functions
    
    pub async fn authorize_user(self: &Self, user_id: i64, token: &str) -> bool {
        let row = sqlx::query!(
            "SELECT COUNT(1) FROM users WHERE user_id = $1 AND api_token = $2",
            user_id,
            token,
        )
        .fetch_one(&self.pool)
        .await;
        row.is_ok()
    }
    pub async fn authorize_bot(self: &Self, bot_id: i64, token: &str) -> bool {
        let row = sqlx::query!(
            "SELECT COUNT(1) FROM bots WHERE bot_id = $1 AND api_token = $2",
            bot_id,
            token,
        )
        .fetch_one(&self.pool)
        .await;
        row.is_ok()
    }
}
