use crate::database;
use actix_web::{error::ResponseError, http, HttpResponse};
use bevy_reflect::{Reflect, Struct};
use indexmap::{indexmap, IndexMap};
use log::debug;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use serenity::model::id::ChannelId;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct User {
    pub id: String,
    pub username: String,
    pub disc: String,
    pub avatar: String,
    pub bot: bool,
    pub status: Status,
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default,
)]
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

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default,
)]
#[repr(i32)]
pub enum Flags {
    #[default]
    Unlocked = 0,
    EditLocked = 1,
    StaffLocked = 2,
    StatsLocked = 3,
    VoteLocked = 4,
    System = 5,
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
#[repr(i32)]
pub enum UserState {
    #[default]
    Normal = 0,
    GlobalBan = 1,
    ProfileEditBan = 2,
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
#[repr(i32)]
pub enum CommandType {
    #[default]
    PrefixCommand = 0,
    SlashCommandGlobal = 1,
    SlashCommandGuild = 2,
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
#[repr(i32)]
pub enum LongDescriptionType {
    Html = 0,
    #[default]
    MarkdownServerSide = 1,
}

#[derive(
    Eq, TryFromPrimitive, Serialize, Deserialize, PartialEq, Clone, Copy, Default, Reflect
)]
#[repr(i32)]
pub enum ImportSource {
    Rdl,
    #[default]
    Other
}

// A import source item - a bot list that can be imported from
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ImportSourceListItem {
    pub id: ImportSource,
    pub name: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ImportSourceList {
    pub sources: Vec<ImportSourceListItem>,    
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
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
    pub flags: Vec<i32>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Tag {
    pub name: String,
    pub iconify_data: String,
    pub id: String,
    pub owner_guild: Option<String>,
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub viewed_as: String,
    pub description: String,
}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
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
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
        }
    }
}
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ResolvedPackBot {
    pub user: User,
    pub description: String,
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
pub struct VoteBotQuery {
    pub test: bool,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct GetUserBotPath {
    pub user_id: i64,
    pub bot_id: i64,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct ImportQuery {
    pub src: ImportSource,
}


#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct GetUserServerPath {
    pub user_id: i64,
    pub server_id: i64,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct GetUserPackPath {
    pub user_id: i64,
    pub pack_id: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct PreviewRequest {
    pub text: String,
    pub long_description_type: LongDescriptionType,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct PreviewResponse {
    pub preview: String,
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
    pub avatar: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct OauthUserLogin {
    pub state: UserState,
    pub token: String,
    pub user: User,
    pub site_lang: String,
    pub css: Option<String>,
}

/// Bot Stats
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotStats {
    pub guild_count: i64,
    pub shard_count: Option<i64>,
    pub shards: Option<Vec<i32>>,
    pub user_count: Option<i64>,
}

/// The response from the oauth2 endpoint. We do not care about anything but access token
#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct OauthAccessTokenResponse {
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct Empty {}

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
    pub target_id: String,
}

// Internal Secrets Struct
#[derive(Deserialize)]
pub struct Secrets {
    pub client_id: String,
    pub client_secret: String,
    pub token_main: String,
    pub token_server: String,
    pub japi_key: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ListStats {
    pub total_bots: i64,
    pub total_servers: i64,
    pub total_users: i64,
    pub bots: Vec<IndexBot>,
    pub servers: Vec<IndexBot>,
    pub uptime: f64,
    pub cpu_idle: f64,
    pub mem_total: u64,
    pub mem_free: u64,
    pub mem_available: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub mem_dirty: u64,
    pub mem_active: u64,
    pub mem_inactive: u64,
    pub mem_buffers: u64,
    pub mem_committed: u64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Partner {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub image: String,
    pub description: String,
    pub links: IndexMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Partners {
    pub partners: Vec<Partner>,
    pub icons: IndexMap<String, String>,
}

impl Default for Partners {
    fn default() -> Self {
        Partners {
            partners: vec![Partner {
                id: "0".to_string(),
                name: "My development".to_string(),
                owner: "12345678901234567".to_string(),
                image: "".to_string(),
                description: "Some random description".to_string(),
                links: indexmap![
                    "discord".to_string() => "https://discord.com/lmao".to_string(),
                    "website".to_string() => "https://example.com".to_string(),
                ],
            }],
            icons: IndexMap::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DiscordChannels {
    pub bot_logs: ChannelId,
    pub appeals_channel: ChannelId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DiscordRoles {
    pub staff_ping_add_role: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DiscordData {
    pub channels: DiscordChannels,
    pub roles: DiscordRoles,
}

pub struct AppConfig {
    pub secrets: Secrets,
    pub policies: Policies,
    pub partners: Partners,
    pub discord: DiscordData,
    pub discord_http: serenity::http::Http,
    pub discord_http_server: serenity::http::Http,
}

impl Default for AppConfig {
    fn default() -> Self {
        let path = match env::var_os("HOME") {
            None => {
                panic!("$HOME not set");
            }
            Some(path) => PathBuf::from(path),
        };

        let data_dir = path.into_os_string().into_string().unwrap() + "/FatesList/config/data/";

        debug!("Data dir: {}", data_dir);

        // open secrets.json, handle config
        let mut file =
            File::open(data_dir.to_owned() + "secrets.json").expect("No config file found");
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let secrets: Secrets = serde_json::from_str(&data).expect("JSON was not well-formatted");

        // open policy.json, handle config
        let mut file =
            File::open(data_dir.to_owned() + "policy.json").expect("No policy.json file found");
        let mut policies = String::new();
        file.read_to_string(&mut policies).unwrap();

        let policies: Policies =
            serde_json::from_str(&policies).expect("JSON was not well-formatted");

        // open partners.json, handle config
        let mut file =
            File::open(data_dir.to_owned() + "partners.json").expect("No partners.json file found");
        let mut partners = String::new();
        file.read_to_string(&mut partners).unwrap();

        let partners: Partners =
            serde_json::from_str(&partners).expect("JSON was not well-formatted");

        // open discord.json, handle config
        let mut file =
            File::open(data_dir.to_owned() + "discord.json").expect("No discord.json file found");
        let mut discord = String::new();
        file.read_to_string(&mut discord).unwrap();

        let discord: DiscordData = serde_json::from_str(&discord).expect("Discord data is invalid");

        let token_main = secrets.token_main.clone();
        let token_server = secrets.token_server.clone();

        AppConfig {
            secrets,
            policies,
            partners,
            discord,
            discord_http: serenity::http::Http::new_with_token(&token_main),
            discord_http_server: serenity::http::Http::new_with_token(&token_server),
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

#[derive(Deserialize, Serialize, Default, Reflect)]
pub struct ReviewDeletePath {
    pub rid: String,
}

#[derive(Deserialize, Serialize, Default, Reflect, Clone)]
pub struct FetchBotPath {
    pub id: i64,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotCommand {
    pub cmd_type: CommandType,
    pub groups: Vec<String>,
    pub name: String,
    pub vote_locked: bool,
    pub description: String,
    pub args: Vec<String>,
    pub examples: Vec<String>,
    pub premium_only: bool,
    pub notes: Vec<String>,
    pub doc_link: String,
    pub id: Option<String>,
    pub nsfw: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotCommandVec {
    pub commands: Vec<BotCommand>,
}

#[derive(Deserialize, Serialize, Default, Reflect, Clone)]
pub struct CommandDeleteQuery {
    pub nuke: Option<bool>,
    pub names: Option<String>,
    pub ids: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Resource {
    pub id: Option<String>,
    pub resource_title: String,
    pub resource_link: String,
    pub resource_description: String,
}

#[derive(Deserialize, Serialize, Clone, Default, Reflect)]
pub struct ResourceDeleteQuery {
    pub id: String,
    pub target_type: TargetType,
}

#[derive(Deserialize, Serialize, Clone, Default, Reflect)]
pub struct TargetQuery {
    pub target_type: TargetType,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotOwner {
    pub user: User,
    pub main: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ActionLog {
    pub user_id: String,
    pub bot_id: String,
    pub action: i32,
    pub action_time: chrono::DateTime<chrono::Utc>,
    pub context: Option<String>,
}

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
    pub invite_link: Option<String>,
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
            invite_link: None,
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
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

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
#[repr(i32)]
pub enum WebhookType {
    #[default]
    Vote = 0,
    DiscordIntegration = 1,
    DeprecatedFatesClient = 2,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotSettingsContext {
    pub tags: Vec<Tag>,
    pub features: Vec<Feature>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotSettings {
    pub bot: Bot,
    pub context: BotSettingsContext,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Bot {
    pub user: User,
    pub description: String,
    pub tags: Vec<Tag>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated_at: chrono::DateTime<chrono::Utc>,
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
    pub vpm: Option<Vec<VotesPerMonth>>,
    pub uptime_checks_total: Option<i32>,
    pub uptime_checks_failed: Option<i32>,
    pub commands: IndexMap<String, Vec<BotCommand>>,
    pub resources: Vec<Resource>,
    pub webhook: Option<String>,
    pub webhook_secret: Option<String>,
    pub webhook_type: Option<WebhookType>,
    pub webhook_hmac_only: Option<bool>,
    pub api_token: Option<String>,
}

impl Default for Bot {
    fn default() -> Self {
        let owners = vec![BotOwner::default()];

        let features = vec![Feature::default()];

        let action_logs = vec![ActionLog {
            user_id: "".to_string(),
            bot_id: "".to_string(),
            action: 0,
            action_time: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            context: None,
        }];

        Bot {
            user: User::default(),
            description: "".to_string(),
            tags: Vec::new(),
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            last_updated_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            last_stats_post: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            long_description: "blah blah blah".to_string(),
            long_description_raw: "blah blah blah unsanitized".to_string(),
            long_description_type: LongDescriptionType::MarkdownServerSide,
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
            vpm: Some(vec![VotesPerMonth::default()]),
            uptime_checks_total: Some(30),
            uptime_checks_failed: Some(19),
            commands: IndexMap::from([("default".to_string(), vec![BotCommand::default()])]),
            resources: vec![Resource::default()],
            webhook: Some("This will be redacted for Get Bot endpoint".to_string()),
            webhook_type: None,
            webhook_hmac_only: None,
            webhook_secret: Some("This will be redacted for Get Bot endpoint".to_string()),
            api_token: Some("This will be redacted for Get Bot endpoint".to_string()),
        }
    }
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default,
)]
#[repr(i32)]
pub enum EventName {
    #[default]
    BotVote = 0,
    BotEdit = 2,   // Not sent anymore
    BotDelete = 3, // Not sent anymore
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

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildInviteBaypawData {
    pub url: String,
    pub cid: u64, // First successful cid
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default,
)]
#[repr(i32)]
pub enum UserBotAction {
    #[default]
    Approve = 0,
    Deny = 1,
    Certify = 2,
    Ban = 3,
    Claim = 4,
    Unclaim = 5,
    TransferOwnership = 6,
    EditBot = 7,
    DeleteBot = 8,
    Unban = 9,
    Uncertify = 10,
    Unverify = 11,
    Requeue = 12,
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default,
)]
#[repr(i32)]
pub enum BotRequestType {
    #[default]
    Appeal = 0,
    Certification = 1,
}

// {"m": {"e": enums.APIEvents.bot_view}, "ctx": {"user": str(user_id), "widget": False, "vote_page": compact}}

// TODO: Make analytics actually work
#[derive(Deserialize, Serialize, Clone)]
pub struct BotViewProp {
    pub widget: bool,
    pub vote_page: bool,
    pub invite: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BotVoteProp {
    pub test: bool,
    pub votes: i64,
}

#[derive(Eq, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default, Reflect)]
#[repr(i32)]
pub enum TargetType {
    #[default]
    Bot = 0,
    Server = 1,
}

impl TargetType {
    pub fn to_arg(t: TargetType) -> &'static str {
        match t {
            TargetType::Bot => "bot",
            TargetType::Server => "server",
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EventContext {
    pub user: Option<String>,
    pub target: String,
    pub target_type: TargetType,
    pub ts: i64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct EventMeta {
    pub e: EventName,
    pub eid: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Event<T: Serialize + Clone + Sync> {
    pub m: EventMeta,
    pub ctx: EventContext,
    pub props: T,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct VoteWebhookEvent {
    pub id: String,
    pub user: String, // Backwards compatibility
    pub ts: i64,
    pub votes: i64,
    pub eid: String,
    pub test: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Policies {
    rules: IndexMap<String, IndexMap<String, Vec<String>>>,
    privacy_policy: IndexMap<String, IndexMap<String, Vec<String>>>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotRequest {
    pub request_type: BotRequestType,
    pub appeal: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct UserVoted {
    pub votes: i64,
    pub voted: bool,
    pub vote_right_now: bool,
    pub vote_epoch: i64,
    pub time_to_vote: i64,
    pub timestamps: Vec<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct VotesPerMonth {
    pub votes: i64,
    pub ts: chrono::DateTime<chrono::Utc>,
}

impl Default for VotesPerMonth {
    fn default() -> Self {
        Self {
            votes: 0,
            ts: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct JAPIAppDataApp {
    pub id: String,
    pub bot_public: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct JAPIAppDataBot {
    pub id: String,
    pub approximate_guild_count: i64,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct JAPIAppData {
    pub application: JAPIAppDataApp,
    pub bot: JAPIAppDataBot,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct JAPIApplication {
    pub data: JAPIAppData,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Profile {
    pub user: User,
    pub bots: Vec<IndexBot>,
    pub description: String,
    pub profile_css: String,
    pub user_css: String,
    pub vote_reminder_channel: Option<String>,
    pub packs: Vec<BotPack>,
    pub state: UserState,
    pub site_lang: String,
    pub action_logs: Vec<ActionLog>,
    //pub vote_reminders: Option<Vec<String>>,
    //pub vote_reminders_servers: Option<Vec<String>>,
    // TODO: Ack data
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ReviewVote {
    pub user_id: String,
    pub upvote: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ParsedReviewVotes {
    pub votes: Vec<ReviewVote>,
    pub upvotes: Vec<String>,
    pub downvotes: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Review {
    pub id: Option<uuid::Uuid>,
    pub reply: bool,
    pub star_rating: bigdecimal::BigDecimal,
    pub review_text: String,
    pub votes: ParsedReviewVotes,
    pub flagged: bool,
    pub user: User,
    pub epoch: Vec<i64>,
    pub replies: Vec<Review>,
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ReviewStats {
    pub average_stars: bigdecimal::BigDecimal,
    pub total: i64,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ParsedReview {
    pub reviews: Vec<Review>,
    pub per_page: i64,
    pub from: i64,
    pub stats: ReviewStats,
    pub user_review: Option<Review>,
}

#[derive(Deserialize, Serialize, Clone, Reflect)]
pub struct ReviewQuery {
    pub target_type: TargetType,
    pub page: Option<i32>,
    pub user_id: Option<i64>,
}

// Error Handling
pub enum ProfileCheckError {
    SQLError(sqlx::Error),
    InvalidVoteReminderChannel,
}

impl ProfileCheckError {
    pub fn to_string(&self) -> String {
        match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
            Self::InvalidVoteReminderChannel => {
                "Invalid vote reminder channel. Are you sure its a valid channel ID?".to_string()
            }
        }
    }
}

pub enum ResourceAddError {
    SQLError(sqlx::Error),
}

impl ResourceAddError {
    pub fn to_string(&self) -> String {
        match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
        }
    }
}

pub enum CommandAddError {
    SQLError(sqlx::Error),
}

impl fmt::Display for CommandAddError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
        })
    }
}

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("Not Found")]
    NotFoundGeneric,
    #[error("Forbidden")]
    ForbiddenGeneric,
    #[error("Unknown Internal Error")]
    Unknown,
}

#[derive(Debug)]
pub enum GuildInviteError {
    SQLError(sqlx::Error),
    LoginRequired,
    NotAcceptingInvites,
    WhitelistRequired(String),
    Blacklisted,
    StaffReview,
    ServerBanned,
    NoChannelFound,
    RequestError(reqwest::Error),
}

impl fmt::Display for GuildInviteError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(
        &match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
            Self::LoginRequired => "You must login in order to join this server!".to_string(),
            Self::StaffReview => "This server is currently under review by Fates List Staff and not accepting invites at this time!".to_string(),
            Self::NotAcceptingInvites => "This server is private and not accepting invites at this time!".to_string(),
            Self::ServerBanned => "This server has been banned from Fates List. If you are a staff member of this server, contact Fates List Support for more information.".to_string(),
            Self::WhitelistRequired(s) => format!("You need to be whitelisted to join this server!<br/>{}", s),
            Self::Blacklisted => "You have been blacklisted from joining this server!".to_string(),
            Self::NoChannelFound => "Could not find channel to invite you to... Please ask the owner of this server to set an invite or set the invite channel for this server".to_string(),
            Self::RequestError(e) => format!("Error occurred when fetching guild invite from baypaw {}", e),
        })
    }
}

pub enum OauthError {
    BadExchange(reqwest::Error),
    BadExchangeJson(String),
    NoUser(reqwest::Error),
    SQLError(sqlx::Error),
}

impl fmt::Display for OauthError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&match self {
            Self::BadExchange(e) => format!("Bad Exchange: {}", e),
            Self::BadExchangeJson(e) => format!("Bad Exchange JSON: {}", e),
            Self::NoUser(e) => format!("No User: {}", e),
            Self::SQLError(e) => format!("SQL Error: {}", e),
        })
    }
}

pub enum SettingsError {
    NotFound,
    SQLError(sqlx::Error),
}

impl SettingsError {
    pub fn to_string(&self) -> String {
        match self {
            Self::NotFound => "Not Found".to_string(),
            Self::SQLError(e) => format!("SQL error: {}", e),
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum BotActionMode {
    Add,
    Edit,
}

pub enum ReviewAddError {
    SQLError(sqlx::Error),
}

impl fmt::Display for ReviewAddError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
        })
    }
}

pub enum CheckBotError {
    AlreadyExists,
    BotBannedOrDenied(State),
    ClientIDImmutable,
    PrefixTooLong,
    NoVanity,
    VanityTaken,
    InvalidInvitePermNum,
    InvalidInvite,
    ShortDescLengthErr,
    LongDescLengthErr,
    BotNotFound,
    NoTags,
    TooManyTags,
    TooManyFeatures,
    InvalidGithub,
    InvalidPrivacyPolicy,
    InvalidDonate,
    InvalidWebsite,
    BannerCardError(BannerCheckError),
    BannerPageError(BannerCheckError),
    JAPIError(reqwest::Error),
    JAPIDeserError(reqwest::Error),
    ClientIDNeeded,
    InvalidClientID,
    PrivateBot,
    EditLocked,
    OwnerListTooLong,
    OwnerIDParseError,
    OwnerNotFound,
    MainOwnerAddAttempt,
}

impl CheckBotError {
    pub fn to_string(&self) -> String {
        match self {
            Self::AlreadyExists => "This bot already exists on Fates List".to_string(),
            Self::JAPIError(e) => format!("JAPI Error: {}", e),
            Self::JAPIDeserError(e) => format!("JAPI Deserialize Error: {}", e),
            Self::PrivateBot => "This bot is private and cannot be added".to_string(),
            Self::ClientIDNeeded => "Client ID is required for this bot or is incorrect".to_string(),
            Self::InvalidClientID => "Client ID inputted is invalid for this bot".to_string(),
            Self::BotBannedOrDenied(state) => format!("This bot is banned or denied: {:?}", state),
            Self::PrefixTooLong => "Prefix must be shorter than 9 characters".to_string(),
            Self::ClientIDImmutable => "Client ID cannot be changed once set".to_string(),
            Self::NoVanity => "You must have a vanity for your bot. This can be your username. You can prefix it with _ (underscore) if you don't want the extra growth from it. For example _mewbot would disable the mewbot vanity".to_string(),
            Self::VanityTaken => "This vanity has already been taken. Please contact Fates List staff if you wish to report this!".to_string(),
            Self::InvalidInvitePermNum => "This invite is invalid!".to_string(),
            Self::InvalidInvite => "Your invite link must start with https://".to_string(),
            Self::InvalidWebsite => "Your website must start with https://".to_string(),
            Self::ShortDescLengthErr => "Your description must be at least 10 characters long and must be a maximum of 200 characters".to_string(),
            Self::LongDescLengthErr => "Your long description must be at least 200 characters long".to_string(),
            Self::BotNotFound => "According to Discord's API and our cache, your bot does not exist. Please try again after 2 hours.".to_string(),
            Self::NoTags => "You must select tags for your bot".to_string(),
            Self::TooManyTags => "You can only select up to 10 tags for your bot".to_string(),
            Self::TooManyFeatures => "You can only select up to 5 features for your bot".to_string(),
            Self::BannerCardError(e) => format!("{}. Hint: check your banner card", e.to_string()),
            Self::BannerPageError(e) => format!("{}. Hint: check your banner page", e.to_string()),
            Self::InvalidGithub => "Your github must be a valid github link starting with https://www.github.com or https://github.com".to_string(),
            Self::InvalidPrivacyPolicy => "Your privacy policy must be a valid link starting with https:// (note the s), not http://".to_string(),
            Self::InvalidDonate => "Your donate must be a valid link starting with https:// (note the s), not http://".to_string(),
            Self::EditLocked => "This bot has either been locked by staff or has been edit locked by the main owner of the bot".to_string(),
            Self::OwnerListTooLong => "The owner list is too long. You may only have a maximum of 5 extra owners".to_string(),
            Self::OwnerIDParseError => "An owner ID in your owner list is invalid".to_string(),
            Self::OwnerNotFound => "An owner ID in your owner list does not exist".to_string(),
            Self::MainOwnerAddAttempt => "You cannot add a main owner as an extra owner".to_string(),
        }
    }
}

pub enum PackCheckError {
    TooManyBots,
    InvalidBotId,
    TooFewBots,
    InvalidIcon,
    InvalidBanner,
    SQLError(sqlx::Error),
    InvalidPackId,
    DescriptionTooShort,
}

impl PackCheckError {
    pub fn to_string(&self) -> String {
        match self {
            Self::TooManyBots => "You cannot have more than 7 bots in a pack".to_string(),
            Self::InvalidBotId => "One of your bot IDs is invalid".to_string(),
            Self::TooFewBots => {
                "You must have at least 2 bots in a pack. Recheck the Bot IDs?".to_string()
            }
            Self::InvalidIcon => "Your icon must start with https://".to_string(),
            Self::InvalidBanner => "Your icon must start with https://".to_string(),
            Self::SQLError(err) => format!("SQL error: {}", err),
            Self::InvalidPackId => {
                "Your pack ID is invalid. This error should *never* be seen".to_string()
            }
            Self::DescriptionTooShort => {
                "Your description must be at least 10 characters long".to_string()
            }
        }
    }
}

pub enum BannerCheckError {
    BadURL(reqwest::Error),
    StatusError(String),
    BadContentType(String),
}

impl BannerCheckError {
    pub fn to_string(&self) -> String {
        match self {
            Self::BadURL(e) => format!("Bad banner url: {}", e),
            Self::StatusError(s) => format!("Got status code: {} when requesting this banner", s),
            Self::BadContentType(s) => format!(
                "Got invalid content type: {} when requesting this banner",
                s
            ),
        }
    }
}

pub enum VoteBotError {
    Wait(String),
    UnknownError(String),
    SQLError(sqlx::Error),
}

impl VoteBotError {
    pub fn to_string(&self) -> String {
        match self {
            Self::Wait(s) => format!("You must wait {} before voting again", s),
            Self::UnknownError(e) => {
                "An unknown error occurred. Please ask on the Fates List support server: "
                    .to_string()
                    + e
            }
            Self::SQLError(e) => format!("SQL error: {}", e),
        }
    }
}

pub enum StatsError {
    BadStats(String), // TODO
    Locked,
    SQLError(sqlx::Error),
}

impl StatsError {
    pub fn to_string(&self) -> String {
        match self {
            Self::BadStats(e) => format!("Bad stats caught and flagged: {}", e),
            Self::Locked => "You have been banned from using this API endpoint!".to_string(),
            Self::SQLError(e) => format!("SQL error: {}", e),
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
            Self::NotFoundGeneric => http::StatusCode::NOT_FOUND,
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

#[derive(Clone, PartialEq)]
pub enum RouteAuthType {
    User,
    Bot,
    Server,
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
    pub auth_types: Vec<RouteAuthType>,
}
