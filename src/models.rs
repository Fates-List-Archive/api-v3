use serde::{Deserialize, Serialize};
use bevy_reflect::{Reflect, Struct};
use num_enum::TryFromPrimitive;
use serde_repr::*;
use std::collections::HashMap;
use thiserror::Error;
use crate::database;
use actix_web::{http, HttpResponse, error::ResponseError};
use std::fs::File;
use std::io::Read;
use std::env;
use std::path::PathBuf;
use log::debug;
use reqwest;

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
pub enum UserState {
    #[default]
    Normal = 0,
    GlobalBan = 1,
    ProfileBan = 2,
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

/// IndexBot represents a bot/server on the index page
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

#[derive(Deserialize, Serialize, Clone)]
pub struct BotPack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub banner: String,
    pub resolved_bots: Vec<ResolvedPackBot>,
    pub owner: User,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Default for BotPack {
    fn default() -> Self {
        BotPack {
            id: "0".to_string(),
            name: "".to_string(),
            description: "".to_string(),
            icon: "".to_string(),
            banner: "".to_string(),
            resolved_bots: vec![ResolvedPackBot::default()],
            owner: User::default(),
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc),
        }
    }
}
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ResolvedPackBot {
    pub user: User,
    pub description: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotPackCreate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub banner: String,
    pub bots: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SearchProfile {
    pub banner: String,
    pub description: String,
    pub user: User,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SearchTags {
    pub bots: Vec<Tag>,
    pub servers: Vec<Tag>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Search {
    pub bots: Vec<IndexBot>,
    pub servers: Vec<IndexBot>,
    pub profiles: Vec<SearchProfile>,
    pub packs: Vec<BotPack>,
    pub tags: SearchTags,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct IndexQuery {
    pub target_type: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct OauthDoQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct OauthUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct OauthUserLogin {
    pub state: UserState,
    pub token: String,
    pub user: User,
    pub site_lang: String,
    pub css: Option<String>,
}

/// The response from the oauth2 endpoint. We do not care about anything but access token
#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct OauthAccessTokenResponse {
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct SearchQuery {
    pub q: Option<String>
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

// Internal Secrets Struct
#[derive(Deserialize)]
pub struct Secrets {
    pub client_id: String,
    pub client_secret: String,
}

pub struct AppConfig {
    pub secrets: Secrets,
    pub policies: Policies,
}

impl Default for AppConfig {
    fn default() -> Self {
        let path = match env::var_os("HOME") {
            None => { panic!("$HOME not set"); }
            Some(path) => PathBuf::from(path),
        };    

        let data_dir = path.into_os_string().into_string().unwrap() + "/FatesList/config/data/";

        debug!("Data dir: {}", data_dir);

        // open secrets.json, handle config
        let mut file = File::open(data_dir.to_owned() + "secrets.json").expect("No config file found");
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let secrets: Secrets = serde_json::from_str(&data).expect("JSON was not well-formatted");
    
        // open policy.json, handle config
        let mut file = File::open(data_dir.to_owned() + "policy.json").expect("No policy.json file found");
        let mut policies = String::new();
        file.read_to_string(&mut policies).unwrap();

        let policies: Policies = serde_json::from_str(&policies).expect("JSON was not well-formatted");

        AppConfig {
            secrets,
            policies,
        }
    }
}

pub struct AppState {
    pub database: database::Database,
    pub config: AppConfig,
    pub docs: String,
    pub requests: reqwest::Client,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct APIResponse {
    pub done: bool,
    pub reason: Option<String>,
    pub context: Option<String>, // This is the error itself
}

#[derive(Deserialize, Serialize, Default, Reflect)]
pub struct FetchBotQuery {
    pub lang: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Reflect, Clone)]
pub struct FetchBotPath {
    pub id: i64,
}

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

/*
class Guild(BaseModel):
    """
    Represents a server/guild on Fates List
    """
    user: BaseUser
    description: str | None = None
    tags: list[dict[str, str]]
    long_description_type: enums.LongDescType | None = None
    long_description: str | None = None
    guild_count: int
    invite_amount: int
    total_votes: int
    state: enums.BotState
    website: str | None = None
    css: str | None = None
    votes: int
    vanity: str | None = None
    nsfw: bool
    banner_card: str | None = None
    banner_page: str | None = None
    keep_banner_decor: bool | None = None
    flags: list[int] | None = []
*/

#[derive(Deserialize, Serialize, Clone)]
pub struct Server {
    pub user: User,
    pub description: String,
    pub tags: Vec<Tag>,
    pub long_description_type: LongDescriptionType,
    pub long_description: String,
    pub long_description_raw: String,
    pub vanity: Option<String>,
    pub guild_count: i64,
    pub invite_amount: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub state: State,
    pub flags: Vec<i32>,
    pub css: String,
    pub website: Option<String>,
    pub banner_card: Option<String>,
    pub banner_page: Option<String>,
    pub keep_banner_decor: bool,
    pub nsfw: bool,
    pub votes: i64,
    pub total_votes: i64,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            user: User::default(),
            description: "".to_string(),
            tags: vec![],
            long_description_type: LongDescriptionType::default(),
            long_description: "".to_string(),
            long_description_raw: "".to_string(),
            vanity: None,
            guild_count: 0,
            invite_amount: 0,
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc),
            state: State::default(),
            flags: vec![],
            css: "".to_string(),
            website: None,
            banner_card: None,
            banner_page: None,
            keep_banner_decor: false,
            nsfw: false,
            votes: 0,
            total_votes: 0,
        }
    }
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

#[derive(Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
#[repr(i32)]
pub enum EventName {
    #[default]
    BotVote = 0,
    BotEdit = 2,
    BotDelete = 3,
    BotClaim = 4,
    BotApprove = 5, 
    BotDeny = 6, 
    BotBan = 7, 
    BotUnban = 8, 
    BotRequeue = 9, 
    BotCertify = 10, 
    BotUncertify = 11, 
    BotTransfer = 12, 
    BotUnverify = 15, 
    BotView = 16, 
    BotInvite = 17,
    BotUnclaim = 18,
    BotVoteReset = 20, 
    BotLock = 22, 
    BotUnlock = 23,
    ReviewVote = 30, 
    ReviewAdd = 31, 
    ReviewEdit = 32, 
    ReviewDelete = 33,
    ResourceAdd = 40, 
    ResourceDelete = 41, 
    CommandAdd = 50, 
    CommandDelete = 51,
    ServerView = 70, 
    ServerVote = 71,
    ServerInvite = 72, 
}

// {"m": {"e": enums.APIEvents.bot_view}, "ctx": {"user": str(user_id), "widget": False, "vote_page": compact}}

#[derive(Deserialize, Serialize, Clone)]
pub struct BotViewProp {
    pub widget: bool,
    pub vote_page: bool
}

#[derive(Eq, Serialize, Deserialize, PartialEq, Clone, Copy, Default)]
pub enum EventTargetType {
    #[default]
    Bot,
    Server,
}

impl EventTargetType {
    pub fn to_arg(t: EventTargetType) -> &'static str {
        match t {
            EventTargetType::Bot => "1",
            EventTargetType::Server => "2",
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EventContext {
    pub user: Option<String>,
    pub target: String,
    pub target_type: EventTargetType,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EventMeta {
    pub e: EventName,
    pub eid: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Event<T: Serialize + Clone + Send> {
    pub m: EventMeta,    
    pub ctx: EventContext,
    pub props: T,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Policies {
    rules: PolicyRules,
    privacy_policy: HashMap<String, HashMap<String, Vec<String>>>,
}

// We want sane ordering so we fix it with a struct
#[derive(Deserialize, Serialize, Clone, Default)]
#[repr(C)]
pub struct PolicyRules {
    bot_requirements: HashMap<String, Vec<String>>,
    certification: HashMap<String, Vec<String>>,
}



// Error Handling
#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Not Found")]
    NotFoundGeneric,
    #[error("Forbidden")]
    ForbiddenGeneric,
    #[error("Unknown Internal Error")]
    Unknown,
}

pub enum OauthError {
    BadExchange(reqwest::Error),
    BadExchangeJson(String),
    NoUser(reqwest::Error),
    SQLError(sqlx::Error),
}

impl OauthError {
    pub fn to_string(&self) -> String {
        match self {
            Self::BadExchange(e) => format!("Bad Exchange: {}", e),
            Self::BadExchangeJson(e) => format!("Bad Exchange JSON: {}", e),
            Self::NoUser(e) => format!("No User: {}", e),
            Self::SQLError(e) => format!("SQL Error: {}", e),
        }
    }
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

pub struct Route<'a, T: Serialize, T2: Serialize, T3: Struct + Serialize, T4: Struct + Serialize> {
    pub title: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub path_params: &'a T3,
    pub query_params: &'a T4,
    pub description: &'a str,
    pub request_body: &'a T,
    pub response_body: &'a T2,
    pub equiv_v2_route: &'a str,
}

