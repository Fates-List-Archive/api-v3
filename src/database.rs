use crate::converters;
use crate::inflector::Inflector;
use crate::models;
use crate::ws;
use async_recursion::async_recursion;
use bigdecimal::FromPrimitive;
use chrono::TimeZone;
use chrono::Utc;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::{Config, Runtime};
use indexmap::{indexmap, IndexMap};
use log::{debug, error};
use serde::Serialize;
use serde_json::json;
use serenity::model::prelude::*;
use sqlx::postgres::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::borrow::Cow;
use tokio::task;

pub struct Database {
    pool: PgPool,
    redis: deadpool_redis::Pool,
    requests: reqwest::Client,
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
            requests: reqwest::Client::builder()
                .user_agent("Lightleap/0.1.0")
                .build()
                .unwrap(),
        }
    }

    /// Only call this when absolutely *needed*
    pub fn get_postgres(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn get_user(&self, user_id: i64) -> models::User {
        // First check cache
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("user-cache:".to_string() + &user_id.to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let user: Option<models::User> = serde_json::from_str(&data).unwrap_or(None);
            if user.is_some() {
                return user.unwrap();
            }
        }

        // Then call baypaw (http://localhost:1234/getch/928702343732658256)
        let req = reqwest::Client::builder()
            .user_agent("DiscordBot (https://fateslist.xyz, 0.1) FatesList-Lightleap-WarriorCats")
            .build()
            .unwrap()
            .get("http://localhost:1234/getch/".to_string() + &user_id.to_string())
            .timeout(std::time::Duration::from_secs(30));

        let res = req.send().await.unwrap();

        let user: models::User = res.json().await.unwrap_or_else(|_| models::User {
            id: "".to_string(),
            username: "Unknown User".to_string(),
            status: models::Status::Unknown,
            disc: "0000".to_string(),
            avatar: "https://api.fateslist.xyz/static/botlisticon.webp".to_string(),
            bot: false,
        });

        if user.id.is_empty() {
            conn.set_ex(
                "user-cache:".to_string() + &user_id.to_string(),
                serde_json::to_string(&user).unwrap(),
                60 * 60 * 1,
            )
            .await
            .unwrap_or_else(|_| "".to_string());
        } else {
            conn.set_ex(
                "user-cache:".to_string() + &user_id.to_string(),
                serde_json::to_string(&user).unwrap(),
                60 * 60 * 8,
            )
            .await
            .unwrap_or_else(|_| "".to_string());
        }
        return user;
    }

    // Index cache functions
    pub async fn set_index_bots_to_cache(&self, cache: &models::Index) {
        let mut conn = self.redis.get().await.unwrap();
        conn.set_ex(
            "index-bots".to_string(),
            serde_json::to_string(cache).unwrap(),
            60,
        )
        .await
        .unwrap_or_else(|_| "".to_string());
    }
    pub async fn set_index_servers_to_cache(&self, cache: &models::Index) {
        let mut conn = self.redis.get().await.unwrap();
        conn.set_ex(
            "index-servers".to_string(),
            serde_json::to_string(cache).unwrap(),
            60,
        )
        .await
        .unwrap_or_else(|_| "".to_string());
    }

    pub async fn get_index_bots_from_cache(&self) -> Option<models::Index> {
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("index-bots".to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let bot: Result<models::Index, serde_json::error::Error> = serde_json::from_str(&data);
            match bot {
                Ok(data) => {
                    return Some(data);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        None
    }

    pub async fn get_index_servers_from_cache(&self) -> Option<models::Index> {
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("index-servers".to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let bot: Result<models::Index, serde_json::error::Error> = serde_json::from_str(&data);
            match bot {
                Ok(data) => {
                    return Some(data);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        None
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
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_user(row.bot_id).await,
            };
            bots.push(bot);
        }
        bots
    }

    pub async fn bot_features(&self) -> Vec<models::Feature> {
        let mut features: Vec<models::Feature> = Vec::new();
        let rows = sqlx::query!("SELECT id, name, viewed_as, description FROM features")
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
        }
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
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_user(row.bot_id).await,
            };
            bots.push(bot);
        }
        bots
    }

    pub async fn get_server_user(&self, guild_id: i64) -> models::User {
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
            status: models::Status::Unknown,
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
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(state),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_server_user(row.guild_id).await,
            };
            servers.push(server);
        }
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
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_server_user(row.guild_id).await,
            };
            servers.push(server);
        }
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
        let row = sqlx::query!(
            "SELECT type, redirect FROM vanity WHERE lower(vanity_url) = $1",
            code.to_lowercase()
        )
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(data) => {
                let target_type = match data.r#type {
                    Some(0) => "server",
                    Some(1) => "bot",
                    Some(2) => "profile",
                    _ => "bot",
                };
                let vanity = models::Vanity {
                    target_type: target_type.to_string(),
                    target_id: data.redirect.unwrap_or(0).to_string(),
                };
                Some(vanity)
            }
            Err(_) => None,
        }
    }

    pub async fn get_vanity_from_id(&self, id: i64) -> Option<String> {
        let row = sqlx::query!("SELECT vanity_url FROM vanity WHERE redirect = $1", id)
            .fetch_one(&self.pool)
            .await;
        match row {
            Ok(data) => data.vanity_url,
            Err(_) => None,
        }
    }

    // Auth functions

    pub async fn authorize_user(&self, user_id: i64, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }

        let row = sqlx::query!(
            "SELECT COUNT(*) FROM users WHERE user_id = $1 AND api_token = $2",
            user_id,
            token.replace("User ", ""),
        )
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(count) => count.count.unwrap_or(0) > 0,
            Err(_) => false,
        }
    }
    pub async fn authorize_bot(&self, bot_id: i64, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        let row = sqlx::query!(
            "SELECT COUNT(*) FROM bots WHERE bot_id = $1 AND api_token = $2",
            bot_id,
            token.replace("Bot ", ""),
        )
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(count) => count.count.unwrap_or(0) > 0,
            Err(_) => false,
        }
    }
    pub async fn authorize_server(&self, server_id: i64, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        let row = sqlx::query!(
            "SELECT COUNT(*) FROM servers WHERE guild_id = $1 AND api_token = $2",
            server_id,
            token.replace("Server ", ""),
        )
        .fetch_one(&self.pool)
        .await;
        match row {
            Ok(count) => count.count.unwrap_or(0) > 0,
            Err(_) => false,
        }
    }

    // Cache functions
    pub async fn get_bot_from_cache(&self, bot_id: i64) -> Option<models::Bot> {
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("bot:".to_string() + &bot_id.to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let bot: Result<models::Bot, serde_json::error::Error> = serde_json::from_str(&data);
            match bot {
                Ok(data) => {
                    return Some(data);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        None
    }

    pub async fn get_search_from_cache(&self, query: String) -> Option<models::Search> {
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("search:".to_string() + &query.to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let res: Result<models::Search, serde_json::error::Error> = serde_json::from_str(&data);
            match res {
                Ok(data) => {
                    return Some(data);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        None
    }

    // Cache functions
    pub async fn get_server_from_cache(&self, server_id: i64) -> Option<models::Server> {
        let mut conn = self.redis.get().await.unwrap();
        let data: String = conn
            .get("server:".to_string() + &server_id.to_string())
            .await
            .unwrap_or_else(|_| "".to_string());
        if !data.is_empty() {
            let server: Result<models::Server, serde_json::error::Error> =
                serde_json::from_str(&data);
            match server {
                Ok(data) => {
                    return Some(data);
                }
                Err(_) => {
                    return None;
                }
            }
        }
        None
    }

    // Get bot
    pub async fn get_bot(&self, bot_id: i64) -> Option<models::Bot> {
        let row = sqlx::query!(
            "SELECT bot_id, created_at, last_stats_post, description, 
            css, flags, banner_card, banner_page, guild_count, shard_count, 
            shards, prefix, invite, invite_amount, features, bot_library 
            AS library, state, website, discord AS support, github, 
            user_count, votes, total_votes, donate, privacy_policy,
            nsfw, client_id, uptime_checks_total, uptime_checks_failed, 
            page_style, keep_banner_decor, long_description_type, last_updated_at,
            long_description, webhook_type FROM bots WHERE bot_id = $1 OR 
            client_id = $1",
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
                    }
                    None => {}
                };

                // Sanitize long description
                let long_description_type =
                    models::LongDescriptionType::try_from(data.long_description_type)
                        .unwrap_or(models::LongDescriptionType::MarkdownServerSide);
                let long_description = converters::sanitize_description(
                    long_description_type,
                    data.long_description.clone().unwrap_or_default(),
                );

                // Tags
                let tag_rows = sqlx::query!("SELECT tag FROM bot_tags WHERE bot_id = $1", bot_id)
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();

                let mut tags = Vec::new();

                for tag in tag_rows.iter() {
                    // Get tag info
                    let tag_info =
                        sqlx::query!("SELECT icon FROM bot_list_tags WHERE id = $1", tag.tag,)
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
                    "SELECT owner, main FROM bot_owner WHERE bot_id = $1 ORDER BY main DESC",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();
                let mut owners = Vec::new();
                let mut owners_html = "".to_string();
                for row in owner_rows.iter() {
                    let user = self.get_user(row.owner).await;
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
                        bot_id: bot_id.to_string(),
                        action: action_row.action,
                        action_time: action_row.action_time,
                        context: action_row.context.clone(),
                    })
                }

                // Commands
                let mut commands = IndexMap::new();

                let commands_rows = sqlx::query!(
                    "SELECT id, cmd_type, description, args, examples, 
                    premium_only, notes, doc_link, groups, name, 
                    vote_locked, nsfw FROM bot_commands WHERE bot_id = $1",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();

                debug!("Commands: {:?}", commands_rows);

                for command in commands_rows.iter() {
                    let groups = command.groups.clone();
                    for group in groups {
                        if !commands.contains_key(&group) {
                            debug!("Dropping command key {key}", key = &group.to_string());
                            commands.insert(group.clone(), Vec::new());
                        }
                        commands
                            .get_mut(&group.clone())
                            .unwrap()
                            .push(models::BotCommand {
                                id: Some(command.id.to_string()),
                                nsfw: command.nsfw.unwrap_or(false),
                                cmd_type: models::CommandType::try_from(command.cmd_type)
                                    .unwrap_or(models::CommandType::SlashCommandGlobal),
                                description: command.description.clone().unwrap_or_default(),
                                args: command.args.clone().unwrap_or_default(),
                                examples: command.examples.clone().unwrap_or_default(),
                                premium_only: command.premium_only.unwrap_or_default(),
                                notes: command.notes.clone().unwrap_or_default(),
                                doc_link: command.doc_link.clone().unwrap_or_default(),
                                name: command.name.clone(),
                                vote_locked: command.vote_locked.unwrap_or_default(),
                                groups: command.groups.clone(),
                            });
                    }
                }

                // Resources
                let mut resources = Vec::new();
                let resources_row = sqlx::query!(
                    "SELECT id, resource_title, resource_link, resource_description FROM resources WHERE target_id = $1",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();

                for resource in resources_row.iter() {
                    resources.push(models::Resource {
                        id: Some(resource.id.to_string()),
                        resource_title: resource.resource_title.clone(),
                        resource_link: resource.resource_link.clone(),
                        resource_description: resource.resource_description.clone(),
                    });
                }

                // VPM
                let mut vpm = Vec::new();
                let vpm_row = sqlx::query!(
                    "SELECT votes, epoch FROM bot_stats_votes_pm WHERE bot_id = $1",
                    bot_id
                )
                .fetch_all(&self.pool)
                .await
                .unwrap();

                for row in vpm_row {
                    vpm.push(models::VotesPerMonth {
                        votes: row.votes.unwrap_or(0),
                        ts: Utc
                            .timestamp_opt(row.epoch.unwrap_or(0), 0)
                            .latest()
                            .unwrap_or_else(|| {
                                chrono::DateTime::<chrono::Utc>::from_utc(
                                    chrono::NaiveDateTime::from_timestamp(0, 0),
                                    chrono::Utc,
                                )
                            }),
                    });
                }

                // Make the struct
                let bot = models::Bot {
                    created_at: data.created_at,
                    vpm: Some(vpm),
                    last_stats_post: data.last_stats_post,
                    last_updated_at: data.last_updated_at,
                    description: data
                        .description
                        .unwrap_or_else(|| "No description set".to_string()),
                    css: "<style>".to_string()
                        + &data
                            .css
                            .unwrap_or_else(|| "".to_string())
                            .replace("\\n", "\n")
                            .replace("\\t", "\t")
                        + "</style>",
                    flags: data.flags.unwrap_or_default(),
                    banner_card: data.banner_card,
                    banner_page: data.banner_page,
                    guild_count: data.guild_count.unwrap_or(0),
                    shard_count: data.shard_count.unwrap_or(0),
                    shards: data.shards.unwrap_or_default(),
                    prefix: data.prefix,
                    invite: data.invite.clone(),
                    invite_link: converters::invite_link(
                        client_id.clone(),
                        data.invite.clone().unwrap_or_else(|| "".to_string()),
                    ),
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
                    resources,
                    commands,
                    long_description_type,
                    long_description,
                    long_description_raw: data.long_description.unwrap_or_default(),
                    owners,
                    vanity: self
                        .get_vanity_from_id(bot_id)
                        .await
                        .unwrap_or_else(|| "unknown".to_string()),
                    uptime_checks_total: data.uptime_checks_total,
                    uptime_checks_failed: data.uptime_checks_failed,
                    page_style: models::PageStyle::try_from(data.page_style)
                        .unwrap_or(models::PageStyle::Tabs),
                    user: self.get_user(data.bot_id).await,
                    webhook: None,
                    webhook_secret: None,
                    api_token: None,
                    webhook_hmac_only: None,
                    webhook_type: Some(
                        models::WebhookType::try_from(data.webhook_type.unwrap_or_default())
                            .unwrap_or(models::WebhookType::Vote),
                    ),
                    owners_html,
                    action_logs,
                };

                // This is a good time to update the db's username_cached
                sqlx::query!(
                    "UPDATE bots SET username_cached = $1 WHERE bot_id = $2",
                    bot.user.username.clone(),
                    bot_id
                )
                .execute(&self.pool)
                .await
                .unwrap();

                let mut conn = self.redis.get().await.unwrap();
                conn.set_ex(
                    "bot:".to_string() + &bot_id.to_string(),
                    serde_json::to_string(&bot).unwrap(),
                    60,
                )
                .await
                .unwrap_or_else(|_| "".to_string());
                Some(bot)
            }
            Err(err) => {
                error!("{}", err);
                None
            }
        }
    }

    // Get Server
    pub async fn get_server(&self, server_id: i64) -> Option<models::Server> {
        let data = sqlx::query!(
            "SELECT description, long_description, long_description_type,
            flags, keep_banner_decor, banner_card, banner_page, guild_count, 
            invite_amount, css, state, website, total_votes, votes, nsfw, 
            tags, created_at FROM servers WHERE guild_id = $1",
            server_id
        )
        .fetch_one(&self.pool)
        .await;

        match data {
            Ok(row) => {
                // Sanitize long description
                let long_description_type = models::LongDescriptionType::try_from(
                    row.long_description_type
                        .unwrap_or(models::LongDescriptionType::MarkdownServerSide as i32),
                )
                .unwrap_or(models::LongDescriptionType::MarkdownServerSide);
                let long_description = converters::sanitize_description(
                    long_description_type,
                    row.long_description.clone().unwrap_or_default(),
                );

                // Tags
                let mut tags = Vec::new();

                for tag in row.tags.unwrap_or_default() {
                    let row = sqlx::query!(
                        "SELECT name, id, iconify_data, owner_guild FROM server_tags WHERE id = $1",
                        tag
                    )
                    .fetch_one(&self.pool)
                    .await;
                    match row {
                        Ok(data) => {
                            tags.push(models::Tag {
                                id: data.id,
                                name: data.name,
                                iconify_data: data.iconify_data,
                                owner_guild: Some(data.owner_guild.to_string()),
                            });
                        }
                        Err(err) => {
                            error!("{}", err);
                        }
                    }
                }

                let res = Some(models::Server {
                    flags: row.flags.unwrap_or_default(),
                    description: row
                        .description
                        .unwrap_or_else(|| "No description set".to_string()),
                    long_description,
                    long_description_raw: row.long_description.unwrap_or_default(),
                    long_description_type,
                    banner_card: row.banner_card,
                    banner_page: row.banner_page,
                    keep_banner_decor: row.keep_banner_decor.unwrap_or_default(),
                    guild_count: row.guild_count.unwrap_or_default(),
                    invite_amount: row.invite_amount.unwrap_or_default(),
                    invite_link: None,
                    css: "<style>".to_string() + &row.css.unwrap_or_default() + "</style>",
                    state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                    website: row.website,
                    total_votes: row.total_votes.unwrap_or_default(),
                    votes: row.votes.unwrap_or_default(),
                    nsfw: row.nsfw.unwrap_or(false),
                    created_at: row.created_at,
                    user: self.get_server_user(server_id).await,
                    tags,
                    vanity: self.get_vanity_from_id(server_id).await,
                });

                let mut conn = self.redis.get().await.unwrap();
                conn.set_ex(
                    "server:".to_string() + &server_id.to_string(),
                    serde_json::to_string(&res).unwrap(),
                    60,
                )
                .await
                .unwrap_or_else(|_| "".to_string());

                res
            }
            Err(err) => {
                error!("{}", err);
                None
            }
        }
    }

    pub async fn resolve_pack_bots(&self, bots: Vec<i64>) -> Vec<models::ResolvedPackBot> {
        let mut resolved_bots = Vec::new();
        for bot in bots {
            let description = sqlx::query!("SELECT description FROM bots WHERE bot_id = $1", bot)
                .fetch_one(&self.pool)
                .await;

            if let Ok(desc) = description {
                resolved_bots.push(models::ResolvedPackBot {
                    user: self.get_user(bot).await,
                    description: desc.description.unwrap_or_default(),
                });
            } else {
                // The bot does not exist, maybe deleted? TODO: Delete
            }
        }
        resolved_bots
    }

    pub async fn search(&self, query: String) -> models::Search {
        // Get bots row
        let bots_row = sqlx::query!(
            "SELECT DISTINCT bots.bot_id,
            bots.description, bots.banner_card AS banner, bots.state, 
            bots.votes, bots.flags, bots.guild_count, bots.nsfw FROM bots 
            INNER JOIN bot_owner ON bots.bot_id = bot_owner.bot_id 
            WHERE (bots.description ilike $1 
            OR bots.long_description ilike $1 
            OR bots.username_cached ilike $1 
            OR bot_owner.owner::text ilike $1) 
            AND (bots.state = $2 OR bots.state = $3) 
            ORDER BY bots.votes DESC, bots.guild_count DESC LIMIT 6",
            "%".to_string() + &query + "%",
            models::State::Approved as i32,
            models::State::Certified as i32,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();
        let mut bots = Vec::new();
        for bot in bots_row {
            bots.push(models::IndexBot {
                guild_count: bot.guild_count.unwrap_or_default(),
                description: bot.description.unwrap_or_default(),
                banner: bot.banner.unwrap_or_default(),
                nsfw: bot.nsfw.unwrap_or_default(),
                votes: bot.votes.unwrap_or_default(),
                state: models::State::try_from(bot.state).unwrap_or(models::State::Approved),
                flags: bot.flags.clone().unwrap_or_default(),
                user: self.get_user(bot.bot_id).await,
            });
        }

        // Get servers row
        let servers_row = sqlx::query!(
            "SELECT DISTINCT servers.guild_id,
            servers.description, servers.banner_card AS banner, servers.state,
            servers.votes, servers.guild_count, servers.nsfw, servers.flags FROM servers
            WHERE (servers.description ilike $1
            OR servers.long_description ilike $1
            OR servers.name_cached ilike $1) AND servers.state = $2
            ORDER BY servers.votes DESC, servers.guild_count DESC LIMIT 6",
            "%".to_string() + &query + "%",
            models::State::Approved as i32,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut servers = Vec::new();

        for server in servers_row {
            servers.push(models::IndexBot {
                guild_count: server.guild_count.unwrap_or(0),
                description: server
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: server.banner.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(server.state).unwrap_or(models::State::Approved),
                nsfw: server.nsfw.unwrap_or(false),
                votes: server.votes.unwrap_or(0),
                flags: server.flags.clone().unwrap_or_default(),
                user: self.get_server_user(server.guild_id).await,
            });
        }

        // Profiles
        let profiles_row = sqlx::query!(
            "SELECT DISTINCT users.user_id, users.description FROM users 
            INNER JOIN bot_owner ON users.user_id = bot_owner.owner 
            INNER JOIN bots ON bot_owner.bot_id = bots.bot_id 
            WHERE ((bots.state = 0 OR bots.state = 6) 
            AND (bots.username_cached ilike $1 OR bots.description ilike $1 OR bots.bot_id::text ilike $1)) 
            OR (users.username ilike $1) LIMIT 12", 
            "%".to_string()+&query+"%",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut profiles = Vec::new();

        for profile in profiles_row {
            profiles.push(models::SearchProfile {
                banner: "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string(),
                description: profile.description.unwrap_or_default(),
                user: self.get_user(profile.user_id).await,
            });
        }

        // Tags
        let tags = models::SearchTags {
            bots: self.bot_list_tags().await,
            servers: self.server_list_tags().await,
        };

        // Packs
        let packs_row = sqlx::query!(
            "SELECT DISTINCT bot_packs.id, bot_packs.icon, bot_packs.banner, 
            bot_packs.created_at, bot_packs.owner, bot_packs.bots, 
            bot_packs.description, bot_packs.name FROM (
                SELECT id, icon, banner, 
                created_at, owner, bots, 
                description, name, unnest(bots) AS bot_id FROM bot_packs
            ) bot_packs
            INNER JOIN bots ON bots.bot_id = bot_packs.bot_id 
            INNER JOIN users ON users.user_id = bot_packs.owner
            WHERE bot_packs.name ilike $1 OR bot_packs.owner::text 
            ilike $1 OR users.username ilike $1 OR bots.bot_id::text ilike $1 
            OR bots.username_cached ilike $1",
            "%".to_string() + &query + "%",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut packs = Vec::new();

        for pack in packs_row {
            packs.push(models::BotPack {
                id: pack.id.to_string(),
                name: pack.name.unwrap_or_default().to_string(),
                description: pack.description.unwrap_or_default(),
                icon: pack.icon.unwrap_or_default(),
                banner: pack.banner.unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                owner: self.get_user(pack.owner.unwrap_or_default()).await,
                created_at: pack.created_at.unwrap_or_else(|| {
                    chrono::DateTime::<chrono::Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(0, 0),
                        chrono::Utc,
                    )
                }),
                resolved_bots: self.resolve_pack_bots(pack.bots.unwrap_or_default()).await,
            });
        }

        let res = models::Search {
            bots,
            servers,
            tags,
            profiles,
            packs,
        };

        let mut conn = self.redis.get().await.unwrap();
        conn.set_ex(
            "search:".to_string() + &query.to_string(),
            serde_json::to_string(&res).unwrap(),
            60,
        )
        .await
        .unwrap_or_else(|_| "".to_string());

        res
    }

    // Search bot/server tags
    pub async fn search_tags(&self, tag: String) -> models::Search {
        let rows = sqlx::query!(
            "SELECT DISTINCT bots.bot_id, bots.description, bots.state, bots.banner_card 
            AS banner, bots.flags, bots.votes, bots.guild_count FROM bots INNER JOIN bot_tags 
            ON bot_tags.bot_id = bots.bot_id WHERE bot_tags.tag = $1 AND 
            (
                bots.state = 0 
                OR bots.state = 6
            ) ORDER BY bots.votes DESC LIMIT 15",
            tag
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut bots = Vec::new();

        for row in rows {
            bots.push(models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: false,
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_user(row.bot_id).await,
            });
        }

        let server_rows = sqlx::query!(
            "SELECT DISTINCT guild_id, flags, description, state, banner_card AS banner, 
            votes, guild_count FROM servers WHERE state = 0 AND tags && $1",
            &vec![tag]
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut servers = Vec::new();

        for row in server_rows {
            servers.push(models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: false,
                votes: row.votes.unwrap_or(0),
                flags: row.flags.clone().unwrap_or_default(),
                user: self.get_server_user(row.guild_id).await,
            });
        }

        models::Search {
            bots,
            servers,
            tags: models::SearchTags {
                bots: self.bot_list_tags().await,
                servers: self.server_list_tags().await,
            },
            profiles: Vec::new(), // Not applicable
            packs: Vec::new(),    // Not applicable
        }
    }

    #[async_recursion]
    pub async fn random_bot(&self) -> models::IndexBot {
        let random_row = sqlx::query!(
            "SELECT description, banner_card, state, votes, guild_count, bot_id, flags FROM bots WHERE (state = 0 OR state = 6) AND nsfw = false ORDER BY RANDOM() LIMIT 1"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();
        let index_bot = models::IndexBot {
            description: random_row.description.unwrap_or_default(),
            banner: random_row.banner_card.unwrap_or_else(|| {
                "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
            }),
            state: models::State::try_from(random_row.state).unwrap_or(models::State::Approved),
            nsfw: false,
            votes: random_row.votes.unwrap_or(0),
            guild_count: random_row.guild_count.unwrap_or(0),
            user: self.get_user(random_row.bot_id).await,
            flags: random_row.flags.clone().unwrap_or_default(),
        };
        if index_bot.user.username.starts_with("Deleted") {
            return self.random_bot().await;
        }
        index_bot
    }

    pub async fn random_server(&self) -> models::IndexBot {
        let random_row = sqlx::query!(
            "SELECT description, banner_card, state, votes, guild_count, guild_id, flags FROM servers WHERE (state = 0 OR state = 6) AND nsfw = false ORDER BY RANDOM() LIMIT 1"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap();
        let index_bot = models::IndexBot {
            description: random_row.description.unwrap_or_default(),
            banner: random_row.banner_card.unwrap_or_else(|| {
                "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
            }),
            state: models::State::try_from(random_row.state).unwrap_or(models::State::Approved),
            nsfw: false,
            votes: random_row.votes.unwrap_or(0),
            guild_count: random_row.guild_count.unwrap_or(0),
            user: self.get_server_user(random_row.guild_id).await,
            flags: random_row.flags.clone().unwrap_or_default(),
        };
        index_bot
    }

    pub async fn ws_event<T: 'static + Serialize + Clone + Sync>(&self, event: models::Event<T>) {
        let mut conn = self.redis.get().await.unwrap();
        // Push to required channel
        let hashmap = indexmap![
            event.m.eid.clone() => &event
        ];
        let message: String = serde_json::to_string(&hashmap).unwrap();
        let channel: String =
            models::TargetType::to_arg(event.ctx.target_type).to_owned() + "-" + &event.ctx.target;
        let _: () = conn.publish(channel, message).await.unwrap();

        let target_id = event.ctx.target.parse::<i64>().unwrap();

        let target_type: &str = match event.ctx.target_type {
            models::TargetType::Bot => "bot",
            models::TargetType::Server => "server",
        };

        sqlx::query!(
            "INSERT INTO events (id, type, event) VALUES ($1, $2, $3)",
            target_id,
            target_type,
            json!(hashmap)
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn create_user_oauth(
        &self,
        user: models::OauthUser,
    ) -> Result<models::OauthUserLogin, sqlx::Error> {
        let user_i64 = user.id.parse::<i64>().unwrap();
        let check = sqlx::query!(
            "SELECT state, api_token, user_css, js_allowed, username, site_lang FROM users WHERE user_id = $1",
            user_i64,
        )
        .fetch_one(&self.pool)
        .await;

        let token: String;
        let mut site_lang: Option<String> = Some("en".to_string());
        let mut css: Option<String> = Some("".to_string());
        let mut state = models::UserState::Normal;

        match check {
            Ok(user) => {
                token = user.api_token; // This must always exist, we *should* panic if not
                site_lang = user.site_lang;
                css = user.user_css;
                state =
                    models::UserState::try_from(user.state).unwrap_or(models::UserState::Normal);
            }
            Err(err) => {
                match err {
                    sqlx::Error::RowNotFound => {
                        // We create the new user
                        token = converters::create_token(128);
                        sqlx::query!(
                            "INSERT INTO users (id, user_id, username, user_css, site_lang, api_token) VALUES ($1, $1, $2, $3, $4, $5)",
                            user_i64,
                            user.username,
                            css, // User css is always initially nothing
                            site_lang,
                            token,
                        )
                        .execute(&self.pool)
                        .await
                        .unwrap();
                    }
                    _ => {
                        // Odd error, lets return it
                        error!("{}", err);
                        return Err(err);
                    }
                }
            }
        }

        Ok(models::OauthUserLogin {
            user: models::User {
                id: user.id.clone(),
                username: user.username,
                disc: user.discriminator,
                avatar: user.avatar.unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/botlisticon.webp".to_string()
                }),
                bot: false,
                status: models::Status::Unknown,
            },
            token,
            state,
            site_lang: site_lang.unwrap_or_else(|| "en".to_string()),
            css,
        })
    }

    pub async fn get_user_voted(&self, bot_id: i64, user_id: i64) -> models::UserVoted {
        let voter_ts = sqlx::query!(
            "SELECT timestamps FROM bot_voters WHERE bot_id = $1 AND user_id = $2",
            bot_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await;

        match voter_ts {
            Ok(ts) => {
                let vote_ts = ts.timestamps.unwrap_or_default();

                let votes = vote_ts.len() as i64;

                let mut conn = self.redis.get().await.unwrap();

                // Get vote epoch
                let ttl = conn
                    .ttl(format!("vote_lock:{user_id}", user_id = user_id))
                    .await
                    .unwrap_or(-2);

                let mut time_to_vote: i64 = 0;
                if ttl > 0 {
                    time_to_vote = 60 * 60 * 8 - ttl
                }

                models::UserVoted {
                    votes,
                    vote_epoch: ttl,
                    vote_right_now: ttl < 0,
                    time_to_vote,
                    voted: votes > 0,
                    timestamps: vote_ts,
                }
            }
            Err(_) => models::UserVoted {
                votes: 0,
                time_to_vote: 0,
                vote_right_now: true,
                vote_epoch: -2,
                voted: false,
                timestamps: Vec::new(),
            },
        }
    }

    pub async fn post_stats(
        &self,
        bot_id: i64,
        stats: models::BotStats,
    ) -> Result<(), models::StatsError> {
        // Firstly make sure user does not have the StatsLocked flag
        let bot = self.get_bot(bot_id).await.unwrap();

        if converters::flags_check(bot.flags, vec![models::Flags::StatsLocked as i32]) {
            return Err(models::StatsError::Locked);
        }

        // Shard count
        match stats.shard_count {
            Some(count) => {
                sqlx::query!(
                    "UPDATE bots SET shard_count = $1 WHERE bot_id = $2",
                    count,
                    bot_id
                )
                .execute(&self.pool)
                .await
                .map_err(models::StatsError::SQLError)?;
            }
            None => {
                debug!("Not setting shard_count as it is not provided!")
            }
        }

        match stats.user_count {
            Some(count) => {
                sqlx::query!(
                    "UPDATE bots SET user_count = $1 WHERE bot_id = $2",
                    count,
                    bot_id
                )
                .execute(&self.pool)
                .await
                .map_err(models::StatsError::SQLError)?;
            }
            None => {
                debug!("Not setting user_count as it is not provided!")
            }
        }

        match stats.shards {
            Some(count) => {
                let count_ref: &[i32] = &count;
                sqlx::query!(
                    "UPDATE bots SET shards = $1 WHERE bot_id = $2",
                    count_ref,
                    bot_id
                )
                .execute(&self.pool)
                .await
                .map_err(models::StatsError::SQLError)?;
            }
            None => {
                debug!("Not setting shards as it is not provided!")
            }
        }

        sqlx::query!(
            "UPDATE bots SET last_stats_post = NOW(), guild_count = $1 WHERE bot_id = $2",
            stats.guild_count,
            bot_id,
        )
        .execute(&self.pool)
        .await
        .map_err(models::StatsError::SQLError)?;
        Ok(())
    }

    /// Calls get bot and then fills in api_token, webhook and webhook_secret
    pub async fn get_bot_settings(
        &self,
        bot_id: i64,
    ) -> Result<models::Bot, models::SettingsError> {
        let bot = self
            .get_bot(bot_id)
            .await
            .ok_or(models::SettingsError::NotFound)?;

        let sensitive = sqlx::query!(
            "SELECT api_token, webhook, webhook_secret, webhook_hmac_only
             FROM bots WHERE bot_id = $1",
            bot_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(models::SettingsError::SQLError)?;

        let sensitive_bot = models::Bot {
            api_token: sensitive.api_token,
            webhook: sensitive.webhook,
            webhook_secret: sensitive.webhook_secret,
            webhook_hmac_only: Some(sensitive.webhook_hmac_only.unwrap_or(false)),
            ..bot
        };

        Ok(sensitive_bot)
    }

    pub async fn resolve_guild_invite(
        &self,
        guild_id: i64,
        user_id: i64,
    ) -> Result<String, models::GuildInviteError> {
        // Get state, invite_channel, user_whitelist, user_blacklist, login_required, whitelist_only
        // login_required
        let row = sqlx::query!(
            "SELECT state, invite_channel, user_whitelist, user_blacklist, 
            login_required, whitelist_only, whitelist_form, invite_url
            FROM servers WHERE guild_id = $1",
            guild_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(models::GuildInviteError::SQLError)?;

        let state = row.state;
        let invite_channel = row.invite_channel.unwrap_or(0);
        let user_whitelist = row.user_whitelist.unwrap_or_default();
        let user_blacklist = row.user_blacklist.unwrap_or_default();
        let invite_url = row.invite_url.unwrap_or_else(|| "".to_string());
        let mut login_required = row.login_required.unwrap_or_default();
        let whitelist_only = row.whitelist_only.unwrap_or_default();

        // Whitelist only implies login_required
        if whitelist_only {
            login_required = true
        }

        if login_required && user_id == 0 {
            return Err(models::GuildInviteError::LoginRequired);
        }

        // Get a state
        let state = models::State::try_from(state).unwrap_or(models::State::Approved);

        if state == models::State::Banned {
            return Err(models::GuildInviteError::ServerBanned);
        } else if state == models::State::PrivateStaffOnly {
            return Err(models::GuildInviteError::StaffReview);
        }

        if user_whitelist.contains(&user_id.to_string()) {
            return self
                .invite_resolver(guild_id, user_id, invite_channel, invite_url)
                .await;
        } else {
            if state == models::State::PrivateViewable {
                return Err(models::GuildInviteError::NotAcceptingInvites);
            } else if whitelist_only {
                let form = row.whitelist_form;
                let form_html: String;
                if form.is_none() {
                    form_html = "There is no form to get access to this server!".to_string()
                } else {
                    form_html = format!(
                        "<a href='{}'>You can get acces to this server here</a>",
                        form.unwrap()
                    );
                }
                return Err(models::GuildInviteError::WhitelistRequired(form_html));
            } else if user_blacklist.contains(&user_id.to_string()) {
                return Err(models::GuildInviteError::Blacklisted);
            }
        }

        self.invite_resolver(guild_id, user_id, invite_channel, invite_url)
            .await
    }

    /// Not made for external use outside resolve_guild_invite
    async fn invite_resolver(
        &self,
        guild_id: i64,
        user_id: i64,
        invite_channel: i64,
        invite_url: String,
    ) -> Result<String, models::GuildInviteError> {
        if !invite_url.is_empty() {
            return Ok(invite_url);
        }
        // Call baypaw /guild-invite endpoint
        let req = self
            .requests
            .get(format!(
                "http://127.0.0.1:1234/guild-invite?gid={guild_id}&uid={user_id}&cid={channel_id}",
                guild_id = guild_id,
                user_id = user_id,
                channel_id = invite_channel,
            ))
            .send()
            .await
            .map_err(models::GuildInviteError::RequestError)?;

        if req.status().is_success() {
            // Handle a success here
            let data = req.json::<models::GuildInviteBaypawData>().await;
            if data.is_err() {
                return Err(models::GuildInviteError::RequestError(data.unwrap_err()));
            }
            let data = data.unwrap();
            // Update invite_channel with cid from baypaw
            if data.cid.to_string() != invite_channel.to_string() {
                sqlx::query!(
                    "UPDATE servers SET invite_channel = $1 WHERE guild_id = $2",
                    data.cid as i64,
                    guild_id,
                )
                .execute(&self.pool)
                .await
                .unwrap();
            }
            return Ok(data.url);
        } else {
            return Err(models::GuildInviteError::NoChannelFound);
        }
    }

    // Invite amount updater

    pub async fn update_bot_invite_amount(&self, bot_id: i64) {
        sqlx::query!(
            "UPDATE bots SET invite_amount = invite_amount + 1 WHERE bot_id = $1",
            bot_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn update_server_invite_amount(&self, guild_id: i64) {
        sqlx::query!(
            "UPDATE servers SET invite_amount = invite_amount + 1 WHERE guild_id = $1",
            guild_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    // Security functions

    pub async fn new_bot_token(&self, bot_id: i64) {
        let new_token = converters::create_token(128);
        sqlx::query!(
            "UPDATE bots SET api_token = $1 WHERE bot_id = $2",
            new_token,
            bot_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn new_user_token(&self, user_id: i64) {
        let new_token = converters::create_token(128);
        sqlx::query!(
            "UPDATE users SET api_token = $1 WHERE user_id = $2",
            new_token,
            user_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn new_server_token(&self, server_id: i64) {
        let new_token = converters::create_token(128);
        sqlx::query!(
            "UPDATE servers SET api_token = $1 WHERE guild_id = $2",
            new_token,
            server_id
        )
        .execute(&self.pool)
        .await
        .unwrap();
    }

    pub async fn add_bot(&self, bot: &models::Bot) -> Result<(), sqlx::Error> {
        let id = bot.user.id.parse::<i64>().unwrap();
        let client_id = bot.client_id.parse::<i64>().unwrap_or(id);

        // Step 1: Delete old stale data
        let mut tx = self.pool.begin().await?;
        sqlx::query!("DELETE FROM bots WHERE bot_id = $1", id)
            .execute(&mut tx)
            .await?;
        sqlx::query!("DELETE FROM bot_owner WHERE bot_id = $1", id)
            .execute(&mut tx)
            .await?;
        sqlx::query!("DELETE FROM vanity WHERE redirect = $1", id)
            .execute(&mut tx)
            .await?;
        sqlx::query!("DELETE FROM bot_tags WHERE bot_id = $1", id)
            .execute(&mut tx)
            .await?;

        // Expand features to vec
        let mut features: Vec<String> = Vec::new();
        for feature in &bot.features {
            features.push(feature.id.clone());
        }

        // Step 2: Insert new data
        sqlx::query!(
            "INSERT INTO bots (
            bot_id, prefix, bot_library,
            invite, website, banner_card, banner_page,
            discord, long_description, description,
            api_token, features, long_description_type, 
            css, donate, github,
            webhook, webhook_type, webhook_secret, webhook_hmac_only,
            privacy_policy, nsfw, keep_banner_decor, 
            client_id, guild_count, flags, page_style, id) VALUES(
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 
            $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, 
            $24, $25, $26, $27, $1)",
            id,
            bot.prefix,
            bot.library,
            bot.invite,
            bot.website,
            bot.banner_card,
            bot.banner_page,
            bot.support,
            bot.long_description,
            bot.description,
            converters::create_token(132),
            &features,
            bot.long_description_type as i32,
            bot.css,
            bot.donate,
            bot.github,
            bot.webhook,
            bot.webhook_type.unwrap_or(models::WebhookType::Vote) as i32,
            bot.webhook_secret,
            bot.webhook_hmac_only.unwrap_or(false),
            bot.privacy_policy,
            bot.nsfw,
            bot.keep_banner_decor,
            client_id,
            bot.guild_count,
            &Vec::new(),
            bot.page_style as i32
        )
        .execute(&mut tx)
        .await?;

        // Handle vanity
        sqlx::query!(
            "INSERT INTO vanity (type, vanity_url, redirect) VALUES ($1, $2, $3)",
            1,
            bot.vanity,
            id
        )
        .execute(&mut tx)
        .await?;

        // Handle bot owners
        for owner in &bot.owners {
            sqlx::query!(
                "INSERT INTO bot_owner (bot_id, owner, main) VALUES ($1, $2, $3)",
                id,
                owner.user.id.parse::<i64>().unwrap(),
                owner.main
            )
            .execute(&mut tx)
            .await?;
        }

        // Add bot tags
        for tag in &bot.tags {
            sqlx::query!(
                "INSERT INTO bot_tags (bot_id, tag) VALUES ($1, $2)",
                id,
                tag.id
            )
            .execute(&mut tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn edit_bot(&self, user_id: i64, bot: &models::Bot) -> Result<(), sqlx::Error> {
        let id = bot.user.id.parse::<i64>().unwrap();
        let client_id = bot.client_id.parse::<i64>().unwrap_or(id);

        let mut tx = self.pool.begin().await?;

        // Expand features to vec
        let mut features = Vec::new();
        for feature in &bot.features {
            features.push(feature.id.clone());
        }

        sqlx::query!(
            "UPDATE bots SET bot_library=$2, webhook=$3, description=$4, 
            long_description=$5, prefix=$6, website=$7, discord=$8, 
            banner_card=$9, invite=$10, github = $11, features = $12, 
            long_description_type = $13, webhook_type = $14, css = $15, 
            donate = $16, privacy_policy = $17, nsfw = $18, 
            webhook_secret = $19, webhook_hmac_only = $20,
            banner_page = $21, keep_banner_decor = $22, 
            client_id = $23, page_style = $24, long_description_parsed = null,
            last_updated_at = NOW() WHERE bot_id = $1",
            id,
            bot.library,
            bot.webhook,
            bot.description,
            bot.long_description,
            bot.prefix,
            bot.website,
            bot.support,
            bot.banner_card,
            bot.invite,
            bot.github,
            &features,
            bot.long_description_type as i32,
            bot.webhook_type.unwrap_or(models::WebhookType::Vote) as i32,
            bot.css,
            bot.donate,
            bot.privacy_policy,
            bot.nsfw,
            bot.webhook_secret,
            bot.webhook_hmac_only.unwrap_or(false),
            bot.banner_page,
            bot.keep_banner_decor,
            client_id,
            bot.page_style as i32
        )
        .execute(&mut tx)
        .await?;

        sqlx::query!(
            "DELETE FROM bot_owner WHERE bot_id = $1 AND main = false",
            id
        )
        .execute(&mut tx)
        .await?;

        // Handle bot owners
        for owner in &bot.owners {
            if owner.main {
                continue;
            }
            sqlx::query!(
                "INSERT INTO bot_owner (bot_id, owner, main) VALUES ($1, $2, $3)",
                id,
                owner.user.id.parse::<i64>().unwrap(),
                owner.main
            )
            .execute(&mut tx)
            .await?;
        }

        sqlx::query!("DELETE FROM bot_tags WHERE bot_id = $1", id)
            .execute(&mut tx)
            .await?;

        // Add bot tags
        for tag in &bot.tags {
            sqlx::query!(
                "INSERT INTO bot_tags (bot_id, tag) VALUES ($1, $2)",
                id,
                tag.id
            )
            .execute(&mut tx)
            .await?;
        }

        sqlx::query!(
            "INSERT INTO user_bot_logs (user_id, bot_id, action) VALUES ($1, $2, $3)",
            user_id,
            id,
            models::UserBotAction::EditBot as i32
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn transfer_ownership(&self, prev_owner: i64, bot_id: i64, owner: models::BotOwner) {
        let mut tx = self.pool.begin().await.unwrap();

        // Set old main owner to false
        sqlx::query!(
            "UPDATE bot_owner SET main = false WHERE bot_id = $1 AND main = true",
            bot_id
        )
        .execute(&mut tx)
        .await
        .unwrap();

        // Delete the owner if it exists
        sqlx::query!(
            "DELETE FROM bot_owner WHERE bot_id = $1 AND owner = $2",
            bot_id,
            owner.user.id.parse::<i64>().unwrap()
        )
        .execute(&mut tx)
        .await
        .unwrap();

        // Insert new main owner
        sqlx::query!(
            "INSERT INTO bot_owner (bot_id, owner, main) VALUES ($1, $2, $3)",
            bot_id,
            owner.user.id.parse::<i64>().unwrap(),
            owner.main
        )
        .execute(&mut tx)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO user_bot_logs (user_id, bot_id, action, context) VALUES ($1, $2, $3, $4)",
            prev_owner,
            bot_id,
            models::UserBotAction::TransferOwnership as i32,
            owner.user.id
        )
        .execute(&mut tx)
        .await
        .unwrap();

        tx.commit().await.unwrap();
    }

    pub async fn delete_bot(&self, user_id: i64, bot_id: i64) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!("DELETE FROM bots WHERE bot_id = $1", bot_id)
            .execute(&mut tx)
            .await?;

        sqlx::query!(
            "DELETE FROM vanity WHERE redirect = $1 AND type = 1",
            bot_id
        )
        .execute(&mut tx)
        .await?;

        sqlx::query!(
            "INSERT INTO user_bot_logs (user_id, bot_id, action, context) VALUES ($1, $2, $3, $4)",
            user_id,
            bot_id,
            models::UserBotAction::DeleteBot as i32,
            "".to_string(),
        )
        .execute(&mut tx)
        .await
        .unwrap();

        tx.commit().await?;

        Ok(())
    }

    pub async fn get_pack_owners(&self, pack_id: String) -> Option<i64> {
        let pack_id_uuid = uuid::Uuid::parse_str(&pack_id);

        if let Ok(id) = pack_id_uuid {
            let owners = sqlx::query!("SELECT owner FROM bot_packs WHERE id = $1", id)
                .fetch_one(&self.pool)
                .await;

            if let Ok(owners) = owners {
                return owners.owner;
            } else {
                return None;
            }
        }
        None
    }

    pub async fn add_pack(&self, pack: models::BotPack) -> Result<(), models::PackCheckError> {
        // Get bots from the pack
        let mut bots = Vec::new();
        for bot in pack.resolved_bots {
            let parsed_id = bot.user.id.parse::<i64>();
            if parsed_id.is_err() {
                return Err(models::PackCheckError::InvalidBotId);
            }
            bots.push(parsed_id.unwrap())
        }

        sqlx::query!(
            "INSERT INTO bot_packs (icon, banner, owner, bots, description, name) VALUES ($1, $2, $3, $4, $5, $6)",
            pack.icon, pack.banner, 
            pack.owner.id.parse::<i64>().unwrap(), &bots, 
            pack.description, pack.name
        )
        .execute(&self.pool)
        .await
        .map_err(models::PackCheckError::SQLError)?;

        Ok(())
    }

    pub async fn edit_pack(&self, pack: models::BotPack) -> Result<(), models::PackCheckError> {
        // Get bots from the pack
        let mut bots = Vec::new();
        for bot in pack.resolved_bots {
            let parsed_id = bot.user.id.parse::<i64>();
            if parsed_id.is_err() {
                return Err(models::PackCheckError::InvalidBotId);
            }
            bots.push(parsed_id.unwrap())
        }

        let pack_id_uuid = uuid::Uuid::parse_str(&pack.id);

        if let Ok(id) = pack_id_uuid {
            sqlx::query!(
                "UPDATE bot_packs SET icon = $1, banner = $2, bots = $3, description = $4, name = $5 WHERE id = $6",
                pack.icon, pack.banner,
                &bots, pack.description,
                pack.name, id
            )
            .execute(&self.pool)
            .await
            .map_err(models::PackCheckError::SQLError)?;

            Ok(())
        } else {
            Err(models::PackCheckError::InvalidPackId)
        }
    }

    pub async fn delete_pack(&self, pack_id: String) {
        let pack_id_uuid = uuid::Uuid::parse_str(&pack_id);

        if let Ok(id) = pack_id_uuid {
            sqlx::query!("DELETE FROM bot_packs WHERE id = $1", id)
                .execute(&self.pool)
                .await
                .unwrap();
        }
    }

    pub async fn get_profile(&self, user_id: i64) -> Option<models::Profile> {
        let row = sqlx::query!(
            "SELECT description, site_lang, state, user_css, profile_css, 
            vote_reminder_channel::text FROM users WHERE user_id = $1",
            user_id
        )
        .fetch_one(&self.pool)
        .await;

        if row.is_err() {
            return None;
        }

        let row = row.unwrap();

        let packs_row = sqlx::query!(
            "SELECT id, icon, banner, created_at, owner, bots, description, name FROM bot_packs WHERE owner = $1",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut packs = Vec::new();
        for pack in packs_row {
            packs.push(models::BotPack {
                id: pack.id.to_string(),
                name: pack.name.unwrap_or_default().to_string(),
                description: pack.description.unwrap_or_default(),
                icon: pack.icon.unwrap_or_default(),
                banner: pack.banner.unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                owner: self.get_user(pack.owner.unwrap_or_default()).await,
                created_at: pack.created_at.unwrap_or_else(|| {
                    chrono::DateTime::<chrono::Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(0, 0),
                        chrono::Utc,
                    )
                }),
                resolved_bots: self.resolve_pack_bots(pack.bots.unwrap_or_default()).await,
            });
        }

        let bots_row = sqlx::query!(
            "SELECT DISTINCT bots.bot_id, bots.description, bots.prefix, 
            bots.banner_card AS banner, bots.state, bots.votes, 
            bots.guild_count, bots.nsfw, bots.flags FROM bots 
            INNER JOIN bot_owner ON bot_owner.bot_id = bots.bot_id 
            WHERE bot_owner.owner = $1",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut bots = Vec::new();
        for row in bots_row {
            let bot = models::IndexBot {
                guild_count: row.guild_count.unwrap_or(0),
                description: row
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description set".to_string()),
                banner: row.banner.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                state: models::State::try_from(row.state).unwrap_or(models::State::Approved),
                nsfw: row.nsfw.unwrap_or(false),
                votes: row.votes.unwrap_or(0),
                flags: row.flags.unwrap_or_default(),
                user: self.get_user(row.bot_id).await,
            };
            bots.push(bot);
        }

        // Action logs
        let mut action_logs = Vec::new();
        let action_log_rows = sqlx::query!(
            "SELECT action, bot_id, action_time, context FROM user_bot_logs WHERE user_id = $1",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        for action_row in action_log_rows.iter() {
            action_logs.push(models::ActionLog {
                user_id: user_id.to_string(),
                bot_id: action_row.bot_id.to_string(),
                action: action_row.action,
                action_time: action_row.action_time,
                context: action_row.context.clone(),
            })
        }

        Some(models::Profile {
            bots,
            packs,
            action_logs,
            description: row
                .description
                .unwrap_or_else(|| "This user prefers to be an enigma".to_string()),
            vote_reminder_channel: row.vote_reminder_channel,
            state: models::UserState::try_from(row.state).unwrap_or(models::UserState::Normal),
            user: self.get_user(user_id).await,
            user_css: row.user_css.unwrap_or_default(),
            profile_css: row.profile_css,
            site_lang: row.site_lang.unwrap_or_else(|| "en".to_string()),
        })
    }

    pub async fn update_profile(
        &self,
        user_id: i64,
        profile: models::Profile,
    ) -> Result<(), models::ProfileCheckError> {
        // This only updates profile-editable fields, it does not create packs etc.
        let vote_reminder_channel: Option<i64>;
        if let Some(vrc) = profile.vote_reminder_channel {
            if vrc.is_empty() {
                vote_reminder_channel = None;
            } else {
                let parsed = vrc.parse::<i64>();

                if parsed.is_err() {
                    return Err(models::ProfileCheckError::InvalidVoteReminderChannel);
                }
                vote_reminder_channel = Some(parsed.unwrap());
            }
        } else {
            vote_reminder_channel = None;
        }

        sqlx::query!(
            "UPDATE users SET description = $1, site_lang = $2, 
            user_css = $3, profile_css = $4, vote_reminder_channel = $5 
            WHERE user_id = $6",
            profile.description,
            profile.site_lang,
            profile.user_css,
            profile.profile_css,
            vote_reminder_channel,
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(models::ProfileCheckError::SQLError)?;

        Ok(())
    }

    // Reviews
    #[async_recursion]
    async fn get_review_replies(&self, parent_id: uuid::Uuid) -> Vec<models::Review> {
        let rows = sqlx::query!(
            "SELECT id, user_id, star_rating, epoch, review_text, flagged FROM reviews 
            WHERE parent_id = $1",
            parent_id,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        let mut reviews = Vec::new();

        for row in rows {
            reviews.push(models::Review {
                id: Some(row.id),
                user: self.get_user(row.user_id).await,
                star_rating: row.star_rating,
                epoch: row.epoch,
                votes: self.get_review_votes(row.id).await,
                review_text: row.review_text,
                flagged: row.flagged,
                replies: self.get_review_replies(row.id).await,
                parent_id: Some(parent_id),
                reply: true,
            });
        }

        reviews
    }

    pub async fn get_reviews(
        &self,
        target_id: i64,
        target_type: models::TargetType,
        limit: i64,
        offset: i64,
    ) -> Vec<models::Review> {
        let mut reviews = Vec::new();

        let target_type_num = match target_type {
            models::TargetType::Bot => 0,
            models::TargetType::Server => 1,
        };

        // trim(stringexpression) != ''
        let rows = sqlx::query!(
            "SELECT id, user_id, star_rating, epoch, review_text, flagged FROM reviews 
            WHERE target_id = $1 AND target_type = $2 AND parent_id IS NULL 
            LIMIT $3 OFFSET $4",
            target_id,
            target_type_num,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        for row in rows {
            reviews.push(models::Review {
                id: Some(row.id),
                user: self.get_user(row.user_id).await,
                review_text: row.review_text,
                epoch: row.epoch,
                flagged: row.flagged,
                votes: self.get_review_votes(row.id).await,
                star_rating: row.star_rating,
                replies: self.get_review_replies(row.id).await,
                reply: false,
                parent_id: None,
            });
        }

        reviews
    }

    pub async fn get_review_stats(
        &self,
        target_id: i64,
        target_type: models::TargetType,
    ) -> models::ReviewStats {
        let target_type_num = match target_type {
            models::TargetType::Bot => 0,
            models::TargetType::Server => 1,
        };

        let stats = sqlx::query!(
            "SELECT COUNT(*), AVG(star_rating) AS average_stars FROM reviews WHERE target_id = $1 AND target_type = $2 AND parent_id IS NULL",
            target_id,
            target_type_num
        )
        .fetch_one(&self.pool)
        .await;

        if stats.is_err() {
            error!("Error getting review stats: {}", stats.err().unwrap());
            return models::ReviewStats {
                total: 0,
                average_stars: bigdecimal::BigDecimal::from_i64(0).unwrap(),
            };
        }

        let stats = stats.unwrap();

        models::ReviewStats {
            total: stats.count.unwrap_or_default(),
            average_stars: stats.average_stars.unwrap_or_default(),
        }
    }

    /// Get reviews for *a* user (not replies)
    pub async fn get_reviews_for_user(
        &self,
        user_id: i64,
        target_id: i64,
        target_type: models::TargetType,
    ) -> Option<models::Review> {
        let review = sqlx::query!(
            "SELECT id, review_text, epoch, star_rating, flagged FROM reviews 
            WHERE target_id = $1 AND target_type = $2 AND user_id = $3 AND parent_id 
            IS NULL",
            target_id,
            target_type as i32,
            user_id
        )
        .fetch_one(&self.pool)
        .await;

        if review.is_err() {
            return None;
        }

        let row = review.unwrap();

        return Some(models::Review {
            id: Some(row.id),
            user: self.get_user(user_id).await,
            review_text: row.review_text,
            epoch: row.epoch,
            flagged: row.flagged,
            votes: self.get_review_votes(row.id).await,
            star_rating: row.star_rating,
            replies: self.get_review_replies(row.id).await,
            reply: false,
            parent_id: None,
        });
    }

    pub async fn add_review(
        &self,
        review: models::Review,
        user_id: i64,
        target_id: i64,
        target_type: models::TargetType,
    ) -> Result<(), models::ReviewAddError> {
        let review_id = uuid::Uuid::new_v4();

        let review_type = match target_type {
            models::TargetType::Bot => 0,
            models::TargetType::Server => 1,
        };

        sqlx::query!(
            "INSERT INTO reviews (id, user_id, target_id, target_type, parent_id, 
            star_rating, review_text, flagged) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            review_id,
            user_id,
            target_id,
            review_type,
            review.parent_id,
            review.star_rating,
            review.review_text,
            review.flagged
        )
        .execute(&self.pool)
        .await
        .map_err(models::ReviewAddError::SQLError)?;

        Ok(())
    }

    pub async fn edit_review(&self, review: models::Review) -> Result<(), models::ReviewAddError> {
        sqlx::query!(
            "UPDATE reviews SET star_rating = $1, review_text = $2 WHERE id = $3",
            review.star_rating,
            review.review_text,
            review.id,
        )
        .execute(&self.pool)
        .await
        .map_err(models::ReviewAddError::SQLError)?;

        Ok(())
    }

    /// Get review votes for a review
    pub async fn get_review_votes(&self, review_id: uuid::Uuid) -> models::ParsedReviewVotes {
        let votes = sqlx::query!(
            "SELECT user_id::text, upvote FROM review_votes WHERE id = $1",
            review_id
        )
        .fetch_all(&self.pool)
        .await;

        if votes.is_err() {
            error!("Error getting review votes: {}", votes.err().unwrap());
            return models::ParsedReviewVotes {
                upvotes: Vec::new(),
                downvotes: Vec::new(),
                votes: Vec::new(),
            };
        }
        let votes = votes.unwrap();

        let mut upvotes: Vec<String> = Vec::new();
        let mut downvotes: Vec<String> = Vec::new();
        let mut review_votes = Vec::new();
        for vote in votes {
            let id = vote.user_id.unwrap_or_default();
            if vote.upvote {
                upvotes.push(id.to_string());
            } else {
                downvotes.push(id.to_string());
            }
            review_votes.push(models::ReviewVote {
                user_id: id.to_string(),
                upvote: vote.upvote,
            });
        }

        models::ParsedReviewVotes {
            votes: review_votes,
            upvotes,
            downvotes,
        }
    }

    /// Gets a single review (including replies)
    pub async fn get_single_review(&self, review_id: uuid::Uuid) -> Option<models::Review> {
        let row = sqlx::query!(
            "SELECT id, user_id, review_text, epoch, star_rating, flagged, parent_id 
            FROM reviews WHERE id = $1",
            review_id,
        )
        .fetch_one(&self.pool)
        .await;

        if row.is_err() {
            return None;
        }

        let row = row.unwrap();

        return Some(models::Review {
            id: Some(row.id),
            user: self.get_user(row.user_id).await,
            review_text: row.review_text,
            epoch: row.epoch,
            flagged: row.flagged,
            votes: self.get_review_votes(row.id).await,
            star_rating: row.star_rating,
            replies: Vec::new(),
            reply: false,
            parent_id: row.parent_id,
        });
    }

    pub async fn delete_review(&self, review_id: uuid::Uuid) -> Result<(), models::ReviewAddError> {
        sqlx::query!("DELETE FROM reviews WHERE id = $1", review_id)
            .execute(&self.pool)
            .await
            .map_err(models::ReviewAddError::SQLError)?;

        Ok(())
    }

    pub async fn add_review_vote(
        &self,
        review_id: uuid::Uuid,
        user_id: i64,
        upvote: bool,
    ) -> Result<(), models::ReviewAddError> {
        sqlx::query!(
            "INSERT INTO review_votes (user_id, upvote, id) 
            VALUES ($1, $2, $3) ON CONFLICT (user_id, id) 
            DO UPDATE SET upvote = excluded.upvote;
            ",
            user_id,
            upvote,
            review_id
        )
        .execute(&self.pool)
        .await
        .map_err(models::ReviewAddError::SQLError)?;
        Ok(())
    }

    // Stats functions
    pub async fn get_bot_count(&self) -> i64 {
        let row = sqlx::query!("SELECT COUNT(*) FROM bots")
            .fetch_one(&self.pool)
            .await;

        if row.is_err() {
            return 0;
        }

        let row = row.unwrap();

        return row.count.unwrap();
    }

    pub async fn get_user_count(&self) -> i64 {
        let row = sqlx::query!("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await;

        if row.is_err() {
            return 0;
        }

        let row = row.unwrap();

        return row.count.unwrap();
    }
    pub async fn get_server_count(&self) -> i64 {
        let row = sqlx::query!("SELECT COUNT(*) FROM servers")
            .fetch_one(&self.pool)
            .await;

        if row.is_err() {
            return 0;
        }

        let row = row.unwrap();

        return row.count.unwrap();
    }

    pub async fn get_all_bots(&self) -> Vec<models::IndexBot> {
        let rows = sqlx::query!(
            "SELECT bot_id, username_cached, guild_count, banner_card, 
            description, votes, state, nsfw, flags FROM bots ORDER BY
            votes DESC"
        )
        .fetch_all(&self.pool)
        .await;

        if rows.is_err() {
            return Vec::new();
        }

        let rows = rows.unwrap();

        let mut list = Vec::new();

        for row in rows {
            let state = models::State::try_from(row.state).unwrap_or(models::State::Banned);

            let mut avatar: String =
                "https://api.fateslist.xyz/static/botlisticon.webp".to_string();

            if state == models::State::Certified {
                let user = self.get_user(row.bot_id).await;
                avatar = user.avatar;
            }

            list.push(models::IndexBot {
                user: models::User {
                    id: row.bot_id.to_string(),
                    username: row
                        .username_cached
                        .unwrap_or_else(|| "Unknown User".to_string()),
                    disc: "0000".to_string(), // This is unknown
                    avatar: avatar,           // This is unknown
                    bot: true,
                    status: models::Status::Unknown,
                },
                state: models::State::try_from(row.state).unwrap_or(models::State::Banned),
                description: row
                    .description
                    .unwrap_or_else(|| "No description found!".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                guild_count: row.guild_count.unwrap_or_default(),
                votes: row.votes.unwrap_or_default(),
                nsfw: row.nsfw.unwrap_or_default(),
                flags: row.flags.unwrap_or_default(),
            });
        }

        list
    }

    pub async fn get_all_servers(&self) -> Vec<models::IndexBot> {
        let rows = sqlx::query!(
            "SELECT guild_id, name_cached, guild_count, banner_card, 
            description, votes, state, nsfw, flags FROM servers"
        )
        .fetch_all(&self.pool)
        .await;

        if rows.is_err() {
            return Vec::new();
        }

        let rows = rows.unwrap();

        let mut list = Vec::new();

        for row in rows {
            list.push(models::IndexBot {
                user: models::User {
                    id: row.guild_id.to_string(),
                    username: row.name_cached,
                    disc: "0000".to_string(), // This is unknown
                    avatar: "https://api.fateslist.xyz/static/botlisticon.webp".to_string(), // This is unknown
                    bot: true,
                    status: models::Status::Unknown,
                },
                state: models::State::try_from(row.state).unwrap_or(models::State::Banned),
                description: row
                    .description
                    .unwrap_or_else(|| "No description found!".to_string()),
                banner: row.banner_card.clone().unwrap_or_else(|| {
                    "https://api.fateslist.xyz/static/assets/prod/banner.webp".to_string()
                }),
                guild_count: row.guild_count.unwrap_or_default(),
                votes: row.votes.unwrap_or_default(),
                nsfw: row.nsfw.unwrap_or_default(),
                flags: row.flags.unwrap_or_default(),
            });
        }

        list
    }

    // Resources
    pub async fn add_resource(
        &self,
        target_id: i64,
        target_type: models::TargetType,
        resource: models::Resource,
    ) -> Result<(), models::ResourceAddError> {
        sqlx::query!(
            "INSERT INTO resources (target_id, target_type, 
            resource_title, resource_link, resource_description) 
            VALUES ($1, $2, $3, $4, $5)",
            target_id,
            target_type as i32,
            resource.resource_title,
            resource.resource_link,
            resource.resource_description,
        )
        .execute(&self.pool)
        .await
        .map_err(models::ResourceAddError::SQLError)?;

        Ok(())
    }

    pub async fn delete_resource(
        &self,
        resource_id: uuid::Uuid,
    ) -> Result<(), models::ResourceAddError> {
        sqlx::query!("DELETE FROM resources WHERE id = $1", resource_id,)
            .execute(&self.pool)
            .await
            .map_err(models::ResourceAddError::SQLError)?;

        Ok(())
    }

    pub async fn resource_exists(
        &self,
        resource_id: uuid::Uuid,
        target_id: i64,
        target_type: models::TargetType,
    ) -> bool {
        let row = sqlx::query!(
            "SELECT COUNT(*) FROM resources WHERE
            id = $1 AND target_id = $2 AND target_type = $3",
            resource_id,
            target_id,
            target_type as i32,
        )
        .fetch_one(&self.pool)
        .await;

        if row.is_err() {
            return false;
        }

        let row = row.unwrap();

        return row.count.unwrap_or_default() >= 1;
    }

    // Commands

    /// This takes a &models::BotCommand as we do not need ownership
    pub async fn add_command(
        &self,
        bot_id: i64,
        command: &models::BotCommand,
    ) -> Result<(), models::CommandAddError> {
        sqlx::query!(
            "INSERT INTO bot_commands (bot_id, cmd_type, name, 
            description, args, examples, premium_only, notes, doc_link,
            groups, vote_locked, nsfw) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (name) DO UPDATE SET cmd_type = 
            excluded.cmd_type, description = excluded.description,
            args = excluded.args, examples = excluded.examples,
            premium_only = excluded.premium_only, notes = excluded.notes,
            doc_link = excluded.doc_link, groups = excluded.groups,
            vote_locked = excluded.vote_locked, nsfw = excluded.nsfw",
            bot_id,
            command.cmd_type as i32,
            command.name,
            command.description,
            &command.args,
            &command.examples,
            command.premium_only,
            &command.notes,
            command.doc_link,
            &command.groups,
            command.vote_locked,
            command.nsfw,
        )
        .execute(&self.pool)
        .await
        .map_err(models::CommandAddError::SQLError)?;

        Ok(())
    }

    pub async fn delete_all_commands(&self, id: i64) {
        sqlx::query!("DELETE FROM bot_commands WHERE bot_id = $1", id,)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub async fn delete_commands_by_name(&self, id: i64, name: &str) {
        let err = sqlx::query!(
            "DELETE FROM bot_commands WHERE bot_id = $1 AND name = $2",
            id,
            name
        )
        .execute(&self.pool)
        .await;
        if err.is_err() {
            error!("Failed to delete command {}", name);
        }
    }

    pub async fn delete_commands_by_id(&self, id: i64, cmd_id: uuid::Uuid) {
        let err = sqlx::query!(
            "DELETE FROM bot_commands WHERE bot_id = $1 AND id = $2",
            id,
            cmd_id
        )
        .execute(&self.pool)
        .await;
        if err.is_err() {
            error!("Failed to delete command {}", cmd_id);
        }
    }

    // Vote bot
    #[async_recursion]
    pub async fn vote_bot(
        &self,
        user_id: i64,
        bot_id: i64,
        test: bool,
    ) -> Result<(), models::VoteBotError> {
        if test {
            return self.final_vote_handler_bot(user_id, bot_id, test).await;
        }

        /* Let errors be the thing that tells if a vote has happened

        If INSERT errors, then there is another vote due to unique constraint

        In this case, we error out
        */
        let check = sqlx::query!(
            "INSERT INTO user_vote_table (user_id, bot_id) VALUES ($1, $2)",
            user_id,
            bot_id,
        )
        .execute(&self.pool)
        .await;

        if check.is_err() {
            error!("Failed to insert vote: {}", check.unwrap_err());
            // Check that we actually have a expired vote or not
            let expiry_time = sqlx::query!(
                "SELECT expires_on FROM user_vote_table WHERE user_id = $1 
                AND expires_on < NOW()",
                user_id
            )
            .fetch_one(&self.pool)
            .await;

            if !expiry_time.is_err() {
                sqlx::query!("DELETE FROM user_vote_table WHERE user_id = $1", user_id)
                    .execute(&self.pool)
                    .await
                    .unwrap();
                return self.vote_bot(user_id, bot_id, test).await;
            } else {
                let expiry_time = sqlx::query!(
                    "SELECT expires_on FROM user_vote_table WHERE user_id = $1",
                    user_id
                )
                .fetch_one(&self.pool)
                .await;
                if expiry_time.is_err() {
                    return Err(models::VoteBotError::UnknownError(
                        "Failed to get expiry time".to_string(),
                    ));
                }
                let expiry_time = expiry_time.unwrap().expires_on.unwrap();
                let time_left = expiry_time.timestamp() - chrono::offset::Utc::now().timestamp();
                let seconds = time_left % 60;
                let minutes = (time_left / 60) % 60;
                let hours = (time_left / 60) / 60;
                return Err(models::VoteBotError::Wait(format!(
                    "{} hours, {} minutes, {} seconds",
                    hours, minutes, seconds
                )));
            }
        }

        self.final_vote_handler_bot(user_id, bot_id, test).await
    }

    // Vote server
    #[async_recursion]
    pub async fn vote_server(
        &self,
        discord_server_http: &serenity::http::Http,
        user_id: i64,
        server_id: i64,
        test: bool,
    ) -> Result<(), models::VoteBotError> {
        if test {
            return self
                .final_vote_handler_server(&discord_server_http, user_id, server_id, test)
                .await;
        }

        /* Let errors be the thing that tells if a vote has happened

        If INSERT errors, then there is another vote due to unique constraint

        In this case, we error out
        */
        let check = sqlx::query!(
            "INSERT INTO user_server_vote_table (user_id, guild_id) VALUES ($1, $2)",
            user_id,
            server_id,
        )
        .execute(&self.pool)
        .await;

        if check.is_err() {
            error!("Failed to insert vote: {}", check.unwrap_err());
            // Check that we actually have a expired vote or not
            let expiry_time = sqlx::query!(
                "SELECT expires_on FROM user_server_vote_table WHERE user_id = $1 
                AND expires_on < NOW()",
                user_id
            )
            .fetch_one(&self.pool)
            .await;

            if !expiry_time.is_err() {
                sqlx::query!(
                    "DELETE FROM user_server_vote_table WHERE user_id = $1",
                    user_id
                )
                .execute(&self.pool)
                .await
                .unwrap();
                return self
                    .vote_server(&discord_server_http, user_id, server_id, test)
                    .await;
            } else {
                let expiry_time = sqlx::query!(
                    "SELECT expires_on FROM user_server_vote_table WHERE user_id = $1",
                    user_id
                )
                .fetch_one(&self.pool)
                .await;
                if expiry_time.is_err() {
                    return Err(models::VoteBotError::UnknownError(
                        "Failed to get expiry time".to_string(),
                    ));
                }
                let expiry_time = expiry_time.unwrap().expires_on.unwrap();
                let time_left = expiry_time.timestamp() - chrono::offset::Utc::now().timestamp();
                let seconds = time_left % 60;
                let minutes = (time_left / 60) % 60;
                let hours = (time_left / 60) / 60;
                return Err(models::VoteBotError::Wait(format!(
                    "{} hours, {} minutes, {} seconds",
                    hours, minutes, seconds
                )));
            }
        }

        self.final_vote_handler_server(&discord_server_http, user_id, server_id, test)
            .await
    }

    async fn final_vote_handler_bot(
        &self,
        user_id: i64,
        bot_id: i64,
        test: bool,
    ) -> Result<(), models::VoteBotError> {
        debug!("Test vote: {}", test);
        let mut webhook_user_id = user_id;
        if test {
            webhook_user_id = 519850436899897346;
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(models::VoteBotError::SQLError)?;

        // Add votes
        if !test {
            sqlx::query!(
                "UPDATE bots SET votes = votes + 1, 
                total_votes = total_votes + 1 WHERE bot_id = $1",
                bot_id,
            )
            .execute(&mut tx)
            .await
            .map_err(models::VoteBotError::SQLError)?;

            let row = sqlx::query!(
                "SELECT COUNT(1) FROM bot_voters WHERE user_id = $1 AND bot_id = $2",
                user_id,
                bot_id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(models::VoteBotError::SQLError)?;

            if row.count.unwrap_or_default() == 0 {
                sqlx::query!(
                    "INSERT INTO bot_voters (user_id, bot_id) VALUES ($1, $2)",
                    user_id,
                    bot_id
                )
                .execute(&mut tx)
                .await
                .map_err(models::VoteBotError::SQLError)?;
            } else {
                sqlx::query!(
                    "UPDATE bot_voters SET timestamps = array_append(timestamps, NOW()) WHERE user_id = $1 AND bot_id = $2",
                    user_id,
                    bot_id
                )
                .execute(&mut tx)
                .await
                .map_err(models::VoteBotError::SQLError)?;
            }
        }

        tx.commit().await.map_err(models::VoteBotError::SQLError)?;

        // Send the event here
        let event_id = uuid::Uuid::new_v4();

        // Current votes
        let row = sqlx::query!(
            "SELECT votes, webhook, webhook_secret, webhook_type,
            webhook_hmac_only, api_token FROM bots WHERE bot_id = $1",
            bot_id
        )
        .fetch_one(&self.pool)
        .await;

        if row.is_err() {
            return Err(models::VoteBotError::UnknownError(
                "Failed to get bot".to_string(),
            ));
        }

        let row = row.unwrap();

        // Send vote event over websocket
        let event = models::Event {
            m: models::EventMeta {
                e: models::EventName::BotVote,
                eid: event_id.to_string(),
            },
            ctx: models::EventContext {
                target: bot_id.to_string(),
                target_type: models::TargetType::Bot,
                user: Some(user_id.to_string()),
                ts: chrono::Utc::now().timestamp(),
            },
            props: models::BotVoteProp {
                test,
                votes: row.votes.unwrap_or_default(),
            },
        };
        self.ws_event(event).await;

        // Send vote event over webhook too
        if row.webhook.is_some() {
            let webhook = row.webhook.unwrap();
            let mut webhook_token: String;
            if row.webhook_secret.is_some() {
                webhook_token = row.webhook_secret.unwrap();
                if webhook_token.is_empty() {
                    webhook_token = row.api_token.unwrap();
                }
            } else {
                webhook_token = row.api_token.unwrap();
            }

            let vote_event = models::VoteWebhookEvent {
                eid: event_id.to_string(),
                id: webhook_user_id.clone().to_string(),
                user: webhook_user_id.to_string(),
                votes: row.votes.unwrap_or_default(),
                ts: chrono::Utc::now().timestamp(),
                test,
            };

            if row.webhook_type.is_none() {
                return Err(models::VoteBotError::UnknownError(
                    "Failed to get webhook type".to_string(),
                ));
            }
            let webhook_type = row.webhook_type.unwrap();
            if webhook_type == (models::WebhookType::DiscordIntegration as i32) {
                return Err(models::VoteBotError::UnknownError("Discord integration support is under maintenance. Vote has gone through but you will not recieve any rewards".to_string()));
            } else {
                // Send over webhook
                task::spawn(converters::send_vote_webhook(
                    self.requests.clone(),
                    webhook,
                    webhook_token,
                    row.webhook_hmac_only.unwrap_or(false),
                    vote_event,
                ));
            }
        }

        Ok(())
    }

    async fn final_vote_handler_server(
        &self,
        discord_server_http: &serenity::http::Http,
        user_id: i64,
        server_id: i64,
        test: bool,
    ) -> Result<(), models::VoteBotError> {
        debug!("Test vote: {}", test);
        let mut webhook_user_id = user_id;
        if test {
            webhook_user_id = 519850436899897346;
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(models::VoteBotError::SQLError)?;

        // Add votes
        if !test {
            sqlx::query!(
                "UPDATE servers SET votes = votes + 1, 
                total_votes = total_votes + 1 WHERE guild_id = $1",
                server_id,
            )
            .execute(&mut tx)
            .await
            .map_err(models::VoteBotError::SQLError)?;

            let row = sqlx::query!(
                "SELECT COUNT(1) FROM server_voters WHERE user_id = $1 AND guild_id = $2",
                user_id,
                server_id,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(models::VoteBotError::SQLError)?;

            if row.count.unwrap_or_default() == 0 {
                sqlx::query!(
                    "INSERT INTO server_voters (user_id, guild_id) VALUES ($1, $2)",
                    user_id,
                    server_id
                )
                .execute(&mut tx)
                .await
                .map_err(models::VoteBotError::SQLError)?;
            } else {
                sqlx::query!(
                    "UPDATE server_voters SET timestamps = array_append(timestamps, NOW()) WHERE user_id = $1 AND guild_id = $2",
                    user_id,
                    server_id
                )
                .execute(&mut tx)
                .await
                .map_err(models::VoteBotError::SQLError)?;
            }
        }

        tx.commit().await.map_err(models::VoteBotError::SQLError)?;

        // Send the event here
        let event_id = uuid::Uuid::new_v4();

        // Current votes
        let row = sqlx::query!(
            "SELECT votes, webhook, webhook_secret, webhook_type, autorole_votes,
            webhook_hmac_only, api_token FROM servers WHERE guild_id = $1",
            server_id
        )
        .fetch_one(&self.pool)
        .await;

        if row.is_err() {
            return Err(models::VoteBotError::UnknownError(
                "Failed to get server".to_string(),
            ));
        }

        let row = row.unwrap();

        // Send vote event over websocket
        let event = models::Event {
            m: models::EventMeta {
                e: models::EventName::ServerVote,
                eid: event_id.to_string(),
            },
            ctx: models::EventContext {
                target: server_id.to_string(),
                target_type: models::TargetType::Server,
                user: Some(user_id.to_string()),
                ts: chrono::Utc::now().timestamp(),
            },
            props: models::BotVoteProp {
                test,
                votes: row.votes.unwrap_or_default(),
            },
        };
        self.ws_event(event).await;

        // Send vote event over webhook too
        if row.webhook.is_some() {
            let webhook = row.webhook.unwrap();
            let mut webhook_token: String;
            if row.webhook_secret.is_some() {
                webhook_token = row.webhook_secret.unwrap();
                if webhook_token.is_empty() {
                    webhook_token = row.api_token;
                }
            } else {
                webhook_token = row.api_token;
            }

            let vote_event = models::VoteWebhookEvent {
                eid: event_id.to_string(),
                id: webhook_user_id.clone().to_string(),
                user: webhook_user_id.to_string(),
                votes: row.votes.unwrap_or_default(),
                ts: chrono::Utc::now().timestamp(),
                test,
            };

            if row.webhook_type.is_none() {
                return Err(models::VoteBotError::UnknownError(
                    "Failed to get webhook type".to_string(),
                ));
            }
            let webhook_type = row.webhook_type.unwrap();
            if webhook_type == (models::WebhookType::DiscordIntegration as i32) {
                return Err(models::VoteBotError::UnknownError("Discord integration support is under maintenance. Vote has gone through but you will not recieve any rewards".to_string()));
            } else {
                // Send over webhook
                task::spawn(converters::send_vote_webhook(
                    self.requests.clone(),
                    webhook,
                    webhook_token,
                    row.webhook_hmac_only.unwrap_or(false),
                    vote_event,
                ));
            }
        }

        // Autorole code
        if let Some(autorole_votes) = row.autorole_votes {
            let member = GuildId(server_id as u64)
                .member(&discord_server_http, user_id as u64)
                .await;
            if member.is_err() {
                return Err(models::VoteBotError::UnknownError(
                    "Failed to find you on server for auto roles!".to_string(),
                ));
            }
            let mut member = member.unwrap();
            for role in autorole_votes {
                let res = member
                    .add_role(&discord_server_http, RoleId(role as u64))
                    .await;
                if res.is_err() {
                    error!("Failed to add role {} to user {}", role, user_id);
                }
            }
        }

        Ok(())
    }
}
