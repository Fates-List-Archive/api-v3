use crate::database;
use actix_web::{error::ResponseError, http, HttpResponse};
use log::{debug, error};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::*;
use serenity::model::id::{ChannelId, RoleId, GuildId};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;
use std::collections::HashMap;
use strum_macros::EnumIter;

// Re-export common models
pub use bristlefrost::models::{User, Status, State, UserFlags, Flags, UserState, LongDescriptionType, WebhookType, TargetType};

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default, EnumIter
)]
#[repr(i32)]
pub enum UserExperiments {
    #[default]
    Unknown = 0, // Unknown experiment
    GetRoleSelector = 1, // We switched to native roles
    LynxExperimentRolloutView = 2, // The 'Experiment Rollout' view in lynx
    BotReport = 3, // Bot Reports
    ServerAppealCertification = 4, // Ability to use request type of Appeal or Certification in server appeal
    UserVotePrivacy = 5, // The ability for users to hide their votes from Get Bot Votes and Get Server Votes API
    DevPortal = 6, // The ability for users to access the dev portal. This needs explicit whitelisting and cannot be rolled out
}

impl fmt::Display for UserExperiments {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl UserExperiments {
    pub fn not_enabled(self) -> HttpResponse {
        error!("Experiment {:?} not enabled", self);
        return HttpResponse::UnavailableForLegalReasons().json(APIResponse {
            done: false,
            reason: Some("Experiment not enabled".to_string()),
            context: Some(format!("{:?}", self)),
        });
    }
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, EnumIter
)]
#[repr(i32)]
pub enum Ratelimit {
    Appeal = 30,
}

impl fmt::Display for Ratelimit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct UserExperimentListItem {
    pub name: String,
    pub value: UserExperiments,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ExperimentList {
    pub user_experiments: Vec<UserExperimentListItem>
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default, Debug, EnumIter
)]
#[repr(i32)]
pub enum CommandType {
    #[default]
    PrefixCommand = 0,
    SlashCommandGlobal = 1,
    SlashCommandGuild = 2,
}

#[derive(
    Eq, Serialize, Deserialize, PartialEq, Clone, Copy, Default, Debug, EnumIter
)]
pub enum ImportSource {
    Rdl,
    Ibl,
    Custom,
    #[default]
    Other
}

impl ImportSource {
    pub fn source_name(self) -> String {
        match self {
            ImportSource::Rdl => "Rovel Discord List".to_string(),
            ImportSource::Custom => "Custom Source".to_string(),
            ImportSource::Ibl => "Infinity Bot List".to_string(),
            ImportSource::Other => "Unknown Source".to_string(),
        }
    }
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
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default, Debug, EnumIter
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

#[derive(Deserialize, Serialize, Clone)]
pub struct IndexQuery {
    pub target_type: TargetType,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct VoteBotQuery {
    pub test: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GetUserBotPath {
    pub user_id: i64,
    pub bot_id: i64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ImportQuery {
    pub src: ImportSource,
    pub custom_source: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WsModeStruct {
    pub mode: TargetType,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ImportBody {
    pub ext_data: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GetUserServerPath {
    pub user_id: i64,
    pub server_id: i64,
}

#[derive(Deserialize, Serialize, Clone)]
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

#[derive(Deserialize, Serialize, Clone)]
pub struct OauthDoQuery {
    pub code: String,
    pub state: Option<String>,
    pub frostpaw: bool, // Custom client or not
    pub frostpaw_blood: Option<String>, // Custom client ID
    pub frostpaw_claw: Option<String>, // Custom client hmac data
    pub frostpaw_claw_unseathe_time: Option<i64>, // Custom client reported current time
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct FrostpawClient {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub privacy_policy: String,
    pub secret: Option<String>,
    pub owner: User,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct FrostpawTokenReset {
    pub refresh_token: String,
    pub secret: String
}

#[derive(Deserialize, Serialize, Clone)]
pub struct FrostpawRefreshTokenData {
    pub client_id: String,
    pub user_id: i64,
    pub expires_on: chrono::DateTime<chrono::Utc>
}


#[derive(Deserialize, Serialize, Clone)]
pub struct FrostpawLogin {
    pub client_id: String,
    pub user_id: i64,
    pub token: String, // User token
}

#[derive(Deserialize, Serialize, Clone)]
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
    pub refresh_token: Option<String>,
    pub user: User,
    pub site_lang: String,
    pub css: Option<String>,
    pub user_experiments: Vec<UserExperiments>
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
#[derive(Deserialize, Serialize, Clone)]
pub struct OauthAccessTokenResponse {
    pub access_token: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SearchTagQuery {
    pub q: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SearchQuery {
    pub q: String,
    pub gc_from: i64,
    pub gc_to: i64,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Empty {}

// For the sake of documentation
#[derive(Deserialize, Serialize)]
pub struct VanityPath {
    pub code: String,
}

#[derive(Deserialize, Serialize)]
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
    pub token_squirrelflight: String,
    pub japi_key: String,
    pub ibl_fates_key: String,
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

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct PartnerLinks {
    discord: String,
    website: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Partner {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub image: String,
    pub description: String,
    pub links: PartnerLinks,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Partners {
    pub partners: Vec<Partner>,
    pub icons: PartnerLinks,
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
                links: PartnerLinks {
                    discord: "https://discord.com/lmao".to_string(),
                    website: "https://example.com".to_string(),
                },
            }],
            icons: PartnerLinks::default(),
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
    pub staff_ping_add_role: RoleId,
    pub bot_dev_role: RoleId,
    pub certified_dev_role: RoleId,
    pub i_love_pings_role: RoleId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DiscordServers {
    pub main: GuildId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct DiscordData {
    pub servers: DiscordServers,
    pub channels: DiscordChannels,
    pub roles: DiscordRoles,
}

pub struct AppConfig {
    pub secrets: Secrets,
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
        let token_squirrelflight = secrets.token_squirrelflight.clone();

        AppConfig {
            secrets,
            partners,
            discord,
            discord_http: serenity::http::Http::new_with_token(&token_main),
            discord_http_server: serenity::http::Http::new_with_token(&token_squirrelflight),
        }
    }
}

pub struct AppState {
    pub database: database::Database,
    pub config: AppConfig,
    pub docs: String,
    pub enum_docs: String,
    pub requests: reqwest::Client,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct APIResponse {
    pub done: bool,
    pub reason: Option<String>,
    pub context: Option<String>, // This is the error itself
}

impl APIResponse {
    /// Returns a success API response
    pub fn ok() -> Self {
        APIResponse {
            done: true,
            reason: None,
            context: None,
        }
    }

    /// Returns a failure API response
    /// # Arguments
    /// * `reason` - The reason for the failure
    pub fn err(reason: &dyn ToString) -> Self {
        APIResponse {
            done: false,
            reason: Some(reason.to_string()),
            context: None,
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct ReviewDeletePath {
    pub rid: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct StringIDPath {
    pub id: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
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
    pub doc_link: Option<String>,
    pub id: Option<String>,
    pub nsfw: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BotCommandVec {
    pub commands: Vec<BotCommand>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
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

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ResourceDeleteQuery {
    pub id: String,
    pub target_type: TargetType,
}

#[derive(Deserialize, Serialize, Clone, Default)]
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

impl Default for ActionLog {
    fn default() -> Self {
        ActionLog {
            user_id: "".to_string(),
            bot_id: "".to_string(),
            action: 0,
            action_time: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            context: Some("Some context as to why the action happened".to_string()),
        }        
    }
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
    pub css_raw: String,
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
            vanity: Some("server-vanity".to_string()),
            guild_count: 0,
            invite_amount: 0,
            invite_link: Some("Only present if ``Frostpaw-Invite`` header is set".to_string()),
            created_at: chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(0, 0),
                chrono::Utc,
            ),
            state: State::default(),
            flags: vec![],
            css: "".to_string(),
            css_raw: "unsanitized css".to_string(),
            website: Some("https://frostpaw.com".to_string()),
            banner_card: Some("https://frostpaw.com/assets/img/banner-card.png".to_string()),
            banner_page: Some("https://frostpaw.com/assets/img/banner-page.png".to_string()),
            keep_banner_decor: false,
            nsfw: false,
            votes: 0,
            total_votes: 0,
        }
    }
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
    pub css_raw: String,
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
    pub commands: Vec<BotCommand>,
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

        let action_logs = vec![ActionLog::default()];

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
            prefix: Some("Some prefix, null = slash command".to_string()),
            library: "".to_string(),
            invite: Some("Raw invite, null = auto-generated. Use invite_link instead".to_string()),
            invite_link: "https://discord.com/api/oauth2/authorize....".to_string(),
            invite_amount: 48,
            owners,
            owners_html: "".to_string(),
            features,
            state: State::default(),
            website: Some("https://example.com".to_string()),
            support: Some("".to_string()),
            github: Some("https://github.com/Fates-List/FatesList".to_string()),
            css: "<style></style>".to_string(),
            css_raw: "unsanitized css".to_string(),
            votes: 0,
            total_votes: 0,
            vanity: "".to_string(),
            donate: Some("https://paypal.me/example".to_string()),
            privacy_policy: Some("https://lynx.fateslist.xyz/frostpaw/tos".to_string()),
            nsfw: false,
            banner_card: Some("https://api.fateslist.xyz/static/botlisticon.webp".to_string()),
            banner_page: Some("https://api.fateslist.xyz/static/botlisticon.webp".to_string()),
            keep_banner_decor: false,
            client_id: "".to_string(),
            flags: Vec::new(),
            action_logs,
            vpm: Some(vec![VotesPerMonth::default()]),
            uptime_checks_total: Some(30),
            uptime_checks_failed: Some(19),
            commands: vec![BotCommand::default()],
            resources: vec![Resource::default()],
            webhook: Some("This will be redacted for Get Bot endpoint".to_string()),
            webhook_type: None,
            webhook_hmac_only: None,
            webhook_secret: Some("This (along with ``webhook_type``, ``api_token`` and ``webhook_hmac_only``) will be redacted for Get Bot endpoint".to_string()),
            api_token: Some("This will be redacted for Get Bot endpoint".to_string()),
        }
    }
}

#[derive(
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default, Debug, EnumIter
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
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default, EnumIter
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
    Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Debug, Default, EnumIter
)]
#[repr(i32)]
pub enum AppealType {
    #[default]
    Appeal = 0,
    Certification = 1,
    Report = 2,
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
pub struct Appeal {
    pub request_type: AppealType,
    pub appeal: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct UserVoted {
    pub votes: i64,
    pub voted: bool,
    pub vote_right_now: bool,
    pub expiry: u64,
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
    pub description_raw: String,
    pub description: String,
    pub profile_css: String,
    pub user_css: String,
    pub vote_reminder_channel: Option<String>,
    pub packs: Vec<BotPack>,
    pub state: UserState,
    pub site_lang: String,
    pub action_logs: Vec<ActionLog>,
    pub user_experiments: Vec<UserExperiments>,
    pub flags: Vec<i32>
    // TODO: Ack data
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ParsedReviewVotes {
    pub upvotes: Vec<String>,
    pub downvotes: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ReviewVote {
    pub user_id: String,
    pub upvote: bool,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Review {
    pub id: Option<uuid::Uuid>,
    pub star_rating: bigdecimal::BigDecimal,
    pub review_text: String,
    pub votes: ParsedReviewVotes,
    pub flagged: bool,
    pub user: User,
    pub epoch: Vec<i64>,
    pub replies: Vec<Review>,
    pub parent_id: Option<uuid::Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RoleUpdate {
    pub bot_developer: bool,
    pub certified_developer: bool,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct ReviewQuery {
    pub target_type: TargetType,
    pub page: Option<i32>,
    pub user_id: Option<i64>,
}

// Error Handling
pub enum ProfileCheckError {
    SQLError(sqlx::Error),
    InvalidFlag(i32),
}

impl ProfileCheckError {
    pub fn to_string(&self) -> String {
        match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
            Self::InvalidFlag(f) => format!("Illegal or otherwise non-edittable flag: {}", f),
        }
    }
}

#[derive(Debug)]
pub enum ProfileRolesUpdate {
    SQLError(sqlx::Error),
    MemberNotFound,
    RateLimited(i32),
    DiscordError(serenity::Error),
}

impl ProfileRolesUpdate {
    pub fn to_string(&self) -> String {
        match self {
            Self::SQLError(e) => format!("SQL Error: {}", e),
            Self::MemberNotFound => "You are not on our support server!".to_string(),
            Self::RateLimited(i) => format!("You have been rate limited for {} seconds", i),
            Self::DiscordError(e) => format!("Discord Error: {}", e),
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
    ForbiddenGeneric
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
            Self::ServerBanned => "This server has been banned (or denied due to requiring small changes) from Fates List. If you are a staff member of this server, contact Fates List Support for more information.".to_string(),
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
    Forbidden,
}

impl fmt::Display for CheckBotError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&match self {
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
            Self::Forbidden => "You are not allowed to edit this bot!".to_string(),
        })
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
    AutoroleError,
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
            Self::AutoroleError => "Failed to find you on server for auto roles! This might be because this server does not have Squirrelflight with the 'Manage Roles' permission".to_string(),
        }
    }
}

pub enum StatsError {
    BadStats(String), // TODO
    JAPIError(reqwest::Error),
    JAPIDeserError(reqwest::Error),
    Locked,
    SQLError(sqlx::Error),
    ClientIDNeeded,
}

impl StatsError {
    pub fn to_string(&self) -> String {
        match self {
            Self::BadStats(e) => format!("Bad stats caught and flagged: {}", e),
            Self::JAPIError(e) => format!("Our anti-abuse provider is currently down right now: {}!", e),
            Self::JAPIDeserError(e) => format!("JAPI Deserialize Error: {}", e),
            Self::Locked => "You have been banned from using this API endpoint!".to_string(),
            Self::ClientIDNeeded => "Client ID is required for this bot or is incorrect".to_string(),
            Self::SQLError(e) => format!("SQL error: {}", e),
        }
    }
}

impl CustomError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFoundGeneric => "Not Found".to_string(),
            Self::ForbiddenGeneric => "Forbidden".to_string(),
        }
    }
}

impl ResponseError for CustomError {
    fn status_code(&self) -> http::StatusCode {
        match *self {
            Self::NotFoundGeneric => http::StatusCode::NOT_FOUND,
            Self::ForbiddenGeneric => http::StatusCode::FORBIDDEN,
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

pub struct EnumDesc {
    pub name: &'static str,
    pub description: &'static str,
    pub alt_names: Vec<&'static str>,
    pub gen: fn() -> String,
}

pub struct Route<'a, T: Serialize, T2: Serialize, T3: Serialize, T4: Serialize> {
    pub title: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub path_params: &'a T3,
    pub query_params: &'a T4,
    pub description: &'a str,
    pub request_body: &'a T,
    pub response_body: &'a T2,
    pub auth_types: Vec<RouteAuthType>,
}
