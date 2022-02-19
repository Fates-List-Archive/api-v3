use serde::{Deserialize, Serialize};
use bevy_reflect::Reflect;
use num_enum::TryFromPrimitive;
use serde_repr::*;
use std::collections::HashMap;
use thiserror::Error;
use crate::database;
use actix_web::{http, HttpResponse, error::ResponseError};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct User {
    pub id: String,
    pub username: String,
    pub disc: String,
    pub avatar: String,
    pub bot: bool,
}

#[derive(Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
#[repr(i32)]
pub enum State {
    #[default]
    Approved = 0,
    Pending = 1,
    Denied = 2,
    Hidden = 3,
    Banned = 4,
    UnderReview = 5,
    Certified = 6,
    Archived = 7,
    PrivateViewable = 8,
    PrivateStaffOnly = 9,
}

#[derive(Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
#[repr(i32)]
pub enum CommandType {
    #[default]
    PrefixCommand = 0,
    SlashCommandGlobal = 1,
    SlashCommandGuild = 2,
}

#[derive(Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
#[repr(i32)]
pub enum LongDescriptionType {
    #[default]
    Html = 0,
    MarkdownServerSide = 1, // COMPAT: Maybe make this a subprocess of some form for now if too much breaks or just push to marked?
    MarkdownMarked = 2,
}

#[derive(Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
#[repr(i32)]
pub enum PageStyle {
    #[default]
    Tabs = 0,
    SingleScroll = 1,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IndexBot {
    pub guild_count: i64,
    pub description: String,
    pub banner: String,
    pub nsfw: bool,
    pub votes: i64,
    pub state: State,
    pub user: User,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Tag {
    pub name: String,
    pub iconify_data: String,
    pub id: String,
    pub owner_guild: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub viewed_as: String,
    pub description: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Index {
    pub new: Vec<IndexBot>,
    pub top_voted: Vec<IndexBot>,
    pub certified: Vec<IndexBot>,
    pub tags: Vec<Tag>,
    pub features: Vec<Feature>,
}

impl Index {
    pub fn new() -> Index {
        Index {
            top_voted: Vec::new(),
            certified: Vec::new(),
            new: Vec::new(),
            tags: Vec::new(),
            features: Vec::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct IndexQuery {
    pub target_type: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct Empty {
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Default)]
pub enum Status {
    #[default]
    Unknown = 0,
    Online = 1,
    Offline = 2, // Or invisible
    Idle = 3,
    DoNotDisturb = 4,
}

// For the sake of documentation
#[derive(Deserialize, Serialize, Reflect)]
pub struct VanityPath {
    pub code: String,
}

#[derive(Deserialize, Serialize, Reflect)]
pub struct Vanity {
    pub target_type: String,
    pub target_id: String
}

pub struct AppState {
    pub database: database::Database,
    pub docs: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct APIResponse {
    pub done: bool,
    pub reason: Option<String>,
    pub context: Option<String>, // This is the error itself
}

#[derive(Deserialize, Serialize, Default, Reflect)]
pub struct FetchBotQuery {
    pub no_cache: Option<bool>,
    pub lang: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Reflect)]
pub struct FetchBotPath {
    pub id: i64,
}

/* 
    description: str | None = None
    tags: list[str]
    created_at: datetime.datetime
    last_stats_post: datetime.datetime | None = None
    long_description_type: enums.LongDescType | None = None
    long_description: str | None = None
    guild_count: int
    shard_count: int | None = 0
    user_count: int
    shards: list[int] | None = []
    prefix: str | None = None
    library: str
    invite: str | None = None
    invite_link: str
    invite_amount: int
    owners: BotOwners | None = None
    owners_html: str | None = None
    features: list[BotFeature]
    state: enums.BotState
    page_style: enums.PageStyle | None = enums.PageStyle.tabs
    website: str | None = None
    support: str | None = None
    github: str | None = None
    css: str | None = None
    votes: int
    total_votes: int
    vanity: str | None = "unknown"
    donate: str | None = None
    privacy_policy: str | None = None
    nsfw: bool
    banner_card: str | None = None
    banner_page: str | None = None
    keep_banner_decor: bool | None = None
    client_id: str | None = None
    flags: list[int] | None = []
    action_logs: list[dict] | None = None
    uptime_checks_total: int | None = None
    uptime_checks_failed: int | None = None
    resources: list | None = [] # TODO
    commands: dict | None = {} # TODO
    promos: list[dict] | None = [] # TODO
*/

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotCommand {
    pub cmd_type: CommandType,
    pub cmd_groups: Vec<String>,
    pub cmd_name: String,
    pub vote_locked: bool,
    pub description: String,
    pub args: Vec<String>,
    pub examples: Vec<String>,
    pub premium_only: bool,
    pub notes: Vec<String>,
    pub doc_link: String,
    pub id: String,    
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Resource {
    pub id: String,
    pub resource_title: String,
    pub resource_link: String,
    pub resource_description: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotOwner {
    pub user: User,
    pub main: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ActionLog {
    pub user_id: String,
    pub action: i32,
    pub action_time: chrono::DateTime<chrono::Utc>,
    pub context: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Bot {
    pub user: User,
    pub description: String,
    pub tags: Vec<Tag>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_stats_post: chrono::DateTime<chrono::Utc>,
    pub long_description: String,
    pub long_description_raw: String,
    pub long_description_type: LongDescriptionType,
    pub guild_count: i64,
    pub shard_count: i64,
    pub user_count: i64,
    pub shards: Vec<i32>,
    pub prefix: Option<String>,
    pub library: String,
    pub invite: Option<String>,
    pub invite_link: String,
    pub invite_amount: i32,
    pub owners: Vec<BotOwner>,
    pub owners_html: String,
    pub features: Vec<Feature>,
    pub state: State,
    pub page_style: PageStyle,
    pub website: Option<String>,
    pub support: Option<String>,
    pub github: Option<String>,
    pub css: String,
    pub votes: i64,
    pub total_votes: i64,
    pub vanity: String,
    pub donate: Option<String>,
    pub privacy_policy: Option<String>,
    pub nsfw: bool,
    pub banner_card: Option<String>,
    pub banner_page: Option<String>,
    pub keep_banner_decor: bool,
    pub client_id: String,
    pub flags: Vec<i32>,
    pub action_logs: Vec<ActionLog>,
    pub uptime_checks_total: Option<i32>,
    pub uptime_checks_failed: Option<i32>,
    pub commands: HashMap<String, Vec<BotCommand>>,
    pub resources: Vec<Resource>,
}

impl Default for Bot {
    fn default() -> Self {
        let owners = vec![
            BotOwner::default()
        ];

        let features = vec![
            Feature::default()
        ];

        let action_logs = vec![ActionLog {
            user_id: "".to_string(),
            action: 0,
            action_time: chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc),
            context: None,
        }];

        Bot {
            user: User::default(),
            description: "".to_string(),
            tags: Vec::new(),
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc),
            last_stats_post: chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc),
            long_description: "blah blah blah".to_string(),
            long_description_raw: "blah blah blah unsanitized".to_string(),
            long_description_type: LongDescriptionType::MarkdownMarked,
            page_style: PageStyle::SingleScroll,
            guild_count: 0,
            shard_count: 493,
            user_count: 0,
            shards: Vec::new(),
            prefix: None,
            library: "".to_string(),
            invite: None,
            invite_link: "https://discord.com/api/oauth2/authorize....".to_string(),
            invite_amount: 48,
            owners,
            owners_html: "".to_string(),
            features,
            state: State::default(),
            website: None,
            support: Some("".to_string()),
            github: None,
            css: "<style></style>".to_string(),
            votes: 0,
            total_votes: 0,
            vanity: "".to_string(),
            donate: None,
            privacy_policy: None,
            nsfw: false,
            banner_card: None,
            banner_page: None,
            keep_banner_decor: false,
            client_id: "".to_string(),
            flags: Vec::new(),
            action_logs,
            uptime_checks_total: Some(30),
            uptime_checks_failed: Some(19),
            commands: HashMap::from([
                ("default".to_string(), vec![BotCommand::default()]),
            ]),
            resources: vec![Resource::default()],
        }
    }
}

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Not Found")]
    NotFoundGeneric,
    #[error("Forbidden")]
    ForbiddenGeneric,
    #[error("Unknown Internal Error")]
    Unknown
}

impl CustomError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFoundGeneric => "Not Found".to_string(),
            Self::ForbiddenGeneric => "Forbidden".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> http::StatusCode {
        match *self {
            Self::NotFoundGeneric  => http::StatusCode::NOT_FOUND,
            Self::ForbiddenGeneric => http::StatusCode::FORBIDDEN,
            Self::Unknown => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = APIResponse {
            reason: Some(self.to_string()),
            context: Some(self.name()),
            done: status_code.is_success(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}  