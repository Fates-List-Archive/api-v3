use crate::models;
use crate::docser;
use bigdecimal::FromPrimitive;
use serde::Serialize;
use std::fmt::Debug;
use strum::IntoEnumIterator;
use serde_json::json;

fn doc<T: Serialize, T2: Serialize, T3: Serialize, T4: Serialize>(
    route: models::Route<T, T2, T3, T4>,
) -> String {
    // Serialize request body
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);

    route.request_body.serialize(&mut ser).unwrap();

    // Serialize response body
    let buf2 = Vec::new();
    let formatter2 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser2 = serde_json::Serializer::with_formatter(buf2, formatter2);

    route.response_body.serialize(&mut ser2).unwrap();

    let query_params_str = docser::serialize_docs(route.query_params).unwrap();

    let path_params_str = docser::serialize_docs(route.path_params).unwrap();

    let mut base_doc = format!(
        "### {title}\n#### {method} {path}\n\n{description}",
        title = route.title,
        method = route.method,
        path = route.path,
        description = route.description,
    );

    if !query_params_str.is_empty() {
        base_doc += &("\n\n**Path parameters**\n\n".to_string() + &path_params_str);
    }
    if !query_params_str.is_empty() {
        base_doc += &("\n\n**Query parameters**\n\n".to_string() + &query_params_str);
    }

    let mut auth_needed: String = "".to_string();
    let mut i = 1;
    let auth_lengths = route.auth_types.clone().len();
    for auth in route.auth_types {
        if auth == models::RouteAuthType::Bot {
            auth_needed += "[Bot](https://lynx.fateslist.xyz/docs/endpoints#authorization)";
            if i < auth_lengths {
                auth_needed += ", ";
            }
        } else if auth == models::RouteAuthType::User {
            auth_needed += "[User](https://lynx.fateslist.xyz/docs/endpoints#authorization)";
            if i < auth_lengths {
                auth_needed += ", ";
            }
        } else if auth == models::RouteAuthType::Server {
            auth_needed += "[Server](https://lynx.fateslist.xyz/docs/endpoints#authorization)";
            if i < auth_lengths {
                auth_needed += ", ";
            }
        }
        i += 1;
    }

    return base_doc + &format!(
        "\n\n**Request Body Description**\n\n{request_body_desc}\n\n**Request Body Example**\n\n```json\n{request_body}\n```\n\n**Response Body Description**\n\n{response_body_desc}\n\n**Response Body Example**\n\n```json\n{response_body}\n```\n**Authorization Needed** | {auth_needed}\n\n\n",
        request_body_desc = docser::serialize_docs(route.request_body).unwrap(),
        response_body_desc = docser::serialize_docs(route.response_body).unwrap(),
        request_body = String::from_utf8(ser.into_inner()).unwrap(),
        response_body = String::from_utf8(ser2.into_inner()).unwrap(),
        auth_needed = auth_needed
    );
}

/// Begin a new doc category
fn doc_category(name: &str) -> String {
    format!("## {name}\n\n", name = name)
}

fn enum_doc<T: Debug + Serialize>(typ: T) -> String {
    format!("| **{:?}** | {} |\n", typ, json!(typ))
}

fn new_enum(data: models::EnumDesc) -> String {
    let mut keys = String::new();

    for name in data.alt_names {
        keys += &format!("- ``{}``\n", name);
    }

    format!("
    
### {name}

{desc}

**Common JSON keys**

{keys}

**Values**

| Name | Value |
| :--- | :--- |
{docs}
", 
        name = data.name, 
        desc = data.description, 
        keys = keys, 
        docs = (data.gen)()
    )
}

pub fn document_enums() -> String {
    let mut docs: String = "Below is a reference of all the enums used in Fates List, 
    
It is semi-automatically generated
".to_string();

    // Long Description Type
    docs += &new_enum(models::EnumDesc {
        name: "LongDescriptionType",
        alt_names: vec!["long_description_type"],
        description: "The type of long description that the bot/server has opted for",
        gen: || {
            let mut types = String::new();
            for typ in models::LongDescriptionType::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // State
    docs += &new_enum(models::EnumDesc {
        name: "State",
        alt_names: vec!["state"],
        description: "The state of the bot or server (approved, denied etc.)",
        gen: || {
            let mut types = String::new();
            for typ in models::State::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // UserState
    docs += &new_enum(models::EnumDesc {
        name: "UserState",
        alt_names: vec!["state"],
        description: "The state of the user (normal, banned etc.)",
        gen: || {
            let mut types = String::new();
            for typ in models::UserState::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // Flags
    docs += &new_enum(models::EnumDesc {
        name: "Flags",
        alt_names: vec!["flags"],
        description: "The flags of the bot or server (system bot etc)",
        gen: || {
            let mut types = String::new();
            for typ in models::Flags::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // Experiments
    docs += &new_enum(models::EnumDesc {
        name: "UserExperiments",
        alt_names: vec!["user_experiments"],
        description: "All available user experiments",
        gen: || {
            let mut types = String::new();
            for typ in models::UserExperiments::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // Status
    docs += &new_enum(models::EnumDesc {
        name: "Status",
        alt_names: vec!["flags"],
        description: "The status of the user. **Due to bugs, this currently shows Unknown only but this will be fixed soon!**",
        gen: || {
            let mut types = String::new();
            for typ in models::Status::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // CommandType
    docs += &new_enum(models::EnumDesc {
        name: "CommandType",
        alt_names: vec!["cmd_type"],
        description: "The type of the command being posted (prefix, guild-only etc)",
        gen: || {
            let mut types = String::new();
            for typ in models::CommandType::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // ImportSource
    docs += &new_enum(models::EnumDesc {
        name: "ImportSource",
        alt_names: vec!["src (query parameter)"],
        description: "The source to import bots from",
        gen: || {
            let mut types = String::new();
            for typ in models::ImportSource::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // PageStyle
    docs += &new_enum(models::EnumDesc {
        name: "PageStyle",
        alt_names: vec!["page_style"],
        description: "The style/theme of the bot page. Servers always use single-page view",
        gen: || {
            let mut types = String::new();
            for typ in models::PageStyle::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // WebhookType
    docs += &new_enum(models::EnumDesc {
        name: "WebhookType",
        alt_names: vec!["webhook_type"],
        description: "The type of webhook being used",
        gen: || {
            let mut types = String::new();
            for typ in models::WebhookType::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // EventName
    docs += &new_enum(models::EnumDesc {
        name: "EventName",
        alt_names: vec!["e", "...(non-exhaustive list, use context and it should be self-explanatory)"],
        description: "The name of the event being sent and its corresponding number",
        gen: || {
            let mut types = String::new();
            for typ in models::EventName::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // UserBotAction
    docs += &new_enum(models::EnumDesc {
        name: "UserBotAction",
        alt_names: vec!["action"],
        description: "The name of the event being sent and its corresponding number",
        gen: || {
            let mut types = String::new();
            for typ in models::UserBotAction::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // AppealType
    docs += &new_enum(models::EnumDesc {
        name: "AppealType",
        alt_names: vec!["request_type"],
        description: "The type of appeal being sent",
        gen: || {
            let mut types = String::new();
            for typ in models::AppealType::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    // TargetType
    docs += &new_enum(models::EnumDesc {
        name: "TargetType",
        alt_names: vec!["target_type"],
        description: "The type of the entity (bot/server)",
        gen: || {
            let mut types = String::new();
            for typ in models::TargetType::iter() {
                types += &enum_doc(typ);
            }
            types
        },
    });

    docs
}


pub fn document_routes() -> String {
    let mut docs: String = "**API URL**: ``https://next.fateslist.xyz`` *or* ``https://api.fateslist.xyz`` (for now, can change in future)\n".to_string();

    // Add basic auth stuff
    docs += r#"
## Authorization

- **Bot:** These endpoints require a bot token. 
You can get this from Bot Settings. Make sure to keep this safe and in 
a .gitignore/.env. A prefix of `Bot` before the bot token such as 
`Bot abcdef` is supported and can be used to avoid ambiguity but is not 
required. The default auth scheme if no prefix is given depends on the
endpoint: Endpoints which have only one auth scheme will use that auth 
scheme while endpoints with multiple will always use `Bot` for 
backward compatibility

- **Server:** These endpoints require a server
token which you can get using ``/get API Token`` in your server. 
Same warnings and information from the other authentication types 
apply here. A prefix of ``Server`` before the server token is 
supported and can be used to avoid ambiguity but is not required.

- **User:** These endpoints require a user token. You can get this 
from your profile under the User Token section. If you are using this 
for voting, make sure to allow users to opt out! A prefix of `User` 
before the user token such as `User abcdef` is supported and can be 
used to avoid ambiguity but is not required outside of endpoints that 
have both a user and a bot authentication option such as Get Votes. 
In such endpoints, the default will always be a bot auth unless 
you prefix the token with `User`
"#;

    // API Response route
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);

    models::APIResponse {
        done: true,
        reason: Some("Reason for success of failure, can be null".to_string()),
        context: Some("Any extra context".to_string()),
    }
    .serialize(&mut ser)
    .unwrap();

    docs +=
        &("\n## Base Response\n\nA default API Response will be of the below format:\n\n```json\n"
            .to_string()
            + &String::from_utf8(ser.into_inner()).unwrap()
            + "\n```\n\n");

    // TODO: For each route, add doc system

    docs += &doc_category("Core");

    docs += &doc(
        models::Route {
            title: "Post Stats",
            method: "POST",
            path: "/bots/{id}/stats",
            path_params: &models::FetchBotPath {
                id: 0,
            },
            query_params: &models::Empty {},
            request_body: &models::BotStats {
                guild_count: 3939,
                shard_count: Some(48484),
                shards: Some(vec![149, 22020]),
                user_count: Some(39393),
            },
            response_body: &models::APIResponse::default(),
description: r#"
Post stats to the list

Example:
```py
import requests

# On dpy, guild_count is usually the below
guild_count = len(client.guilds)

# If you are using sharding
shard_count = len(client.shards)
shards = client.shards.keys()

# Optional: User count (this is not accurate for larger bots)
user_count = len(client.users) 

def post_stats(bot_id: int, guild_count: int):
    res = requests.post(f"{api_url}/bots/{bot_id}/stats", json={"guild_count": guild_count})
    json = res.json()
    if res.status != 200:
        # Handle an error in the api
        ...
    return json
```
"#,
            auth_types: vec![models::RouteAuthType::Bot]
    });

    // - Index route
    let index_bots = vec![models::IndexBot::default()];

    let tags = vec![models::Tag::default()];

    let features = vec![models::Feature::default()];

    docs += &doc(models::Route {
        title: "Index",
        method: "GET",
        path: "/index",
        path_params: &models::Empty {},
        query_params: &models::IndexQuery {
            target_type: Some("bot".to_string()),
        },
        description: "Returns the index for bots and servers",
        request_body: &models::Empty {},
        response_body: &models::Index {
            top_voted: index_bots.clone(),
            certified: index_bots.clone(),
            new: index_bots.clone(),
            tags: tags.clone(),
            features: features.clone(),
        },
        auth_types: vec![]
    });

    // Docs Template route
    docs += &doc(models::Route {
        title: "Docs Template",
        method: "GET",
        path: "/_docs_template",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        description: "Internal route to return docs template",
        request_body: &models::Empty {},
        response_body: &models::Empty {},
        auth_types: vec![]
    });

    // Enum Docs Template route
    docs += &doc(models::Route {
        title: "Enum Docs Template",
        method: "GET",
        path: "/_enum_docs_template",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        description: "Internal route to return enum docs template",
        request_body: &models::Empty {},
        response_body: &models::Empty {},
        auth_types: vec![]
    });

    // Experiment List route
    docs += &doc(models::Route {
        title: "Experiment List",
        method: "GET",
        path: "/experiments",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        description: "Returns all currently available experiments",
        request_body: &models::Empty {},
        response_body: &models::ExperimentList {
            user_experiments: vec![models::UserExperimentListItem {
                name: models::UserExperiments::Unknown.to_string(),
                value: models::UserExperiments::Unknown,
            }],
        },
        auth_types: vec![]
    });

    // - Vanity route
    docs += &doc( models::Route {
        title: "Resolve Vanity",
        method: "GET",
        path: "/code/{code}",
        path_params: &models::VanityPath {
            code: "my-vanity".to_string(),
        },
        query_params: &models::Empty {},
        description: "Resolves the vanity for a bot/server in the list",
        request_body: &models::Empty {},
        response_body: &models::Vanity {
            target_id: "0000000000".to_string(),
            target_type: "bot | server".to_string(),
        },
        auth_types: vec![]
    });

    // - Partners route
    docs += &doc( models::Route {
        title: "Get Partners",
        method: "GET",
        path: "/partners",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        description: "Get policies (rules, privacy policy, terms of service)",
        request_body: &models::Empty {},
        response_body: &models::Partners::default(),
        auth_types: vec![]
    });

    // - Preview route
    docs += &doc( models::Route {
        title: "Preview Description",
        method: "WS",
        path: "/ws/_preview",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        description: "Given the preview and long description, parse it and give the sanitized output. You must first connect over websocket!",
        request_body: &models::PreviewRequest::default(),
        response_body: &models::PreviewResponse::default(),
        auth_types: vec![]
    });

    // - Fetch Bot route
    docs += &doc(models::Route {
        title: "Get Bot",
        method: "GET",
        path: "/bots/{id}",
        path_params: &models::FetchBotPath::default(),
        query_params: &models::Empty {},
        description: r#"
Fetches bot information given a bot ID. If not found, 404 will be returned. 

This endpoint handles both bot IDs and client IDs

Differences from API v2:

- Unlike API v2, this does not support compact or no_cache. Owner order is also guaranteed
- *``long_description/css`` is sanitized with ammonia by default, use `long_description_raw` if you want the unsanitized version*
- All responses are cached for a short period of time. There is *no* way to opt out unlike API v2
- Some fields have been renamed or removed (such as ``promos`` which may be readded at a later date)

This API returns some empty fields such as ``webhook``, ``webhook_secret``, ``api_token`` and more. 
This is to allow reuse of the Bot struct in Get Bot Settings which *does* contain this sensitive data. 

**Set the Frostpaw header if you are a custom client. Send Frostpaw-Invite header on invites**
"#,
        request_body: &models::Empty {},
        response_body: &models::Bot::default(), // TODO
        auth_types: vec![],
    });

    // - Search List route
    docs += &doc(models::Route {
        title: "Search List",
        method: "GET",
        path: "/search?q={query}",
        path_params: &models::Empty {},
        query_params: &models::SearchQuery {
            q: "mew".to_string(),
            gc_from: 1,
            gc_to: -1,
        },
        description: r#"
Searches the list based on a query named ``q``. 
        
Using -1 for ``gc_to`` will disable ``gc_to`` field"#,
        request_body: &models::Empty {},
        response_body: &models::Search {
            bots: vec![models::IndexBot::default()],
            servers: vec![models::IndexBot::default()],
            packs: vec![models::BotPack::default()],
            profiles: vec![models::SearchProfile::default()],
            tags: models::SearchTags {
                bots: vec![models::Tag::default()],
                servers: vec![models::Tag::default()]
            },
        },
        auth_types: vec![]
    });

    // - Search Tag route
    docs += &doc(models::Route {
        title: "Search Tag",
        method: "GET",
        path: "/search-tags?q={query}",
        path_params: &models::Empty {},
        query_params: &models::SearchTagQuery {
            q: "music".to_string(),
        },
        description: r#"Searches the list for all bots/servers with tag *exactly* specified ``q``"#,
        request_body: &models::Empty {},
        response_body: &models::Search {
            bots: vec![models::IndexBot::default()],
            servers: vec![models::IndexBot::default()],
            packs: Vec::new(),
            profiles: Vec::new(),
            tags: models::SearchTags {
                bots: vec![models::Tag::default()],
                servers: vec![models::Tag::default()]
            },
        },
        auth_types: vec![]
    });

    docs += &doc(
        models::Route {
            title: "Random Bot",
            method: "GET",
            path: "/random-bot",
            path_params: &models::Empty {},
            query_params: &models::Empty {},
            request_body: &models::Empty {},
            response_body: &models::IndexBot::default(),
description: r#"
Fetches a random bot on the list

Example:
```py
import requests

def random_bot():
    res = requests.get(api_url"/random-bot")
    json = res.json()
    if res.status != 200:
        # Handle an error in the api
        ...
    return json
```
"#,
            auth_types: vec![]
    });

    docs += &doc(
        models::Route {
            title: "Random Server",
            method: "GET",
            path: "/random-server",
            path_params: &models::Empty {},
            query_params: &models::Empty {},
            request_body: &models::Empty {},
            response_body: &models::IndexBot::default(),
description: r#"
Fetches a random server on the list

Example:
```py
import requests

def random_server():
    res = requests.get(api_url"/random-server")
    json = res.json()
    if res.status != 200:
        # Handle an error in the api
        ...
    return json
```
"#,
            auth_types: vec![]
    });

    docs += &doc( models::Route {
        title: "Get Server",
        method: "GET",
        path: "/servers/{id}",
        path_params: &models::FetchBotPath::default(),
        query_params: &models::Empty {},
description: r#"
Fetches server information given a server/guild ID. If not found, 404 will be returned. 

Differences from API v2:

- Unlike API v2, this does not support compact or no_cache.
- *``long_description/css`` is sanitized with ammonia by default, use `long_description_raw` if you want the unsanitized version*
- All responses are cached for a short period of time. There is *no* way to opt out unlike API v2
- Some fields have been renamed or removed
- ``invite_link`` is returned, however is always None unless ``Frostpaw-Invite`` header is set which then pushes you into 
server privacy restrictions. **Note that when fetching invite links, requires login to join is now enabled by default for all new servers**

**Set the Frostpaw header if you are a custom client**
"#,
        request_body: &models::Empty{},
        response_body: &models::Server::default(),
        auth_types: vec![]
    });

    // - Get User Votes
    docs += &doc( models::Route {
        title: "Get Bot Votes",
        method: "GET",
        path: "/users/{user_id}/bots/{bot_id}/votes",
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::Empty {},
description: r#"
Endpoint to check amount of votes a user has.

- votes | The amount of votes the bot has.
- voted | Whether or not the user has *ever* voted for the bot.
- timestamps | A list of timestamps that the user has voted for the bot on that has been recorded.
- expiry | The time when the user can next vote.
- vote_right_now | Whether a user can vote right now. Currently equivalent to `vote_epoch < 0`.

Differences from API v2:

- Unlike API v2, this does not require authorization to use. This is to speed up responses and 
because the last thing people want to scrape are Fates List user votes anyways. **You should not rely on
this however, it is prone to change *anytime* in the future**.
- ``vts`` has been renamed to ``timestamps``
"#,
        request_body: &models::Empty {},
        response_body: &models::UserVoted {
            votes: 10,
            voted: true,
            expiry: 101,
            timestamps: vec![chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc)],
            vote_right_now: false,
        },
        auth_types: vec![]
    });

    // - Create Bot Vote
    docs += &doc(models::Route {
        title: "Create Bot Vote",
        method: "PATCH",
        path: "/users/{user_id}/bots/{bot_id}/votes",
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::VoteBotQuery { test: true },
        description: r#"
This endpoint creates a vote for a bot which can only be done *once* every 8 hours.
"#,
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: false,
            reason: Some("Why the vote failed or any extra info to send to client if the vote succeeded".to_string()),
            context: Some("Some context on the vote".to_string()),
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    // - Create Server Vote
    docs += &doc(models::Route {
        title: "Create Server Vote",
        method: "PATCH",
        path: "/users/{user_id}/servers/{server_id}/votes",
        path_params: &models::GetUserServerPath {
            user_id: 0,
            server_id: 0,
        },
        query_params: &models::VoteBotQuery { test: true },
        description: r#"
This endpoint creates a vote for a server which can only be done *once* every 8 hours
and is independent from a bot vote.
"#,
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: false,
            reason: Some("Why the vote failed or any extra info to send to client if the vote succeeded".to_string()),
            context: Some("Some context on the vote".to_string()),
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Mini Index",
        method: "GET",
        path: "/mini-index",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::Index {
            new: Vec::new(),
            top_voted: Vec::new(),
            certified: Vec::new(),
            tags: tags.clone(),
            features: features.clone(),
        },
        description: r#"
Returns a mini-index which is basically a Index but with only ``tags``
and ``features`` having any data. Other fields are empty arrays/vectors.

This is used internally by sunbeam for the add bot system where a full bot
index is too costly and making a new struct is unnecessary.
"#,
        auth_types: vec![],
    });

    docs += &doc(models::Route {
        title: "Gets Bot Settings",
        method: "GET",
        path: "/users/{user_id}/bots/{bot_id}/settings",
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::BotSettings {
            bot: models::Bot::default(),
            context: models::BotSettingsContext { tags, features },
        },
        description: r#"
Returns the bot settings.

The ``bot`` key here is equivalent to a Get Bot response with the following
differences:

- Sensitive fields (see examples) like ``webhook``, ``api_token``, 
``webhook_secret`` and others are filled out here
- This API only allows bot owners to use it, otherwise it will 400!

Staff members *should* instead use Lynx.

Due to massive changes, this API cannot be mapped onto any v2 API
"#,
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Auth");

    // Oauth Link API
    docs += &doc( models::Route {
        title: "Get OAuth2 Link",
        method: "GET",
        path: "/oauth2",
        description: "Returns the oauth2 link used to login with. ``reason`` contains the state UUID",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: Some("https://discord.com/.........".to_string()),
        },
        auth_types: vec![]
    });

    docs += &doc( models::Route {
        title: "Create OAuth2 Login",
        method: "POST",
        path: "/oauth2",
        description: "Creates a oauth2 login given a code",
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::OauthDoQuery {
            code: "code from discord oauth".to_string(),
            state: Some("Random UUID right now".to_string())
        },
        response_body: &models::OauthUserLogin::default(),
        auth_types: vec![]
    });

    docs += &doc( models::Route {
        title: "Delete OAuth2 Login",
        method: "DELETE",
        path: "/oauth2",
description: r#"
'Deletes' (logs out) a oauth2 login. Always call this when logging out 
even if you do not use cookies as it may perform other logout tasks in future

This API is essentially a logout
"#,
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![]
    });

    docs += &doc_category("Security");

    // New Bot Token
    docs += &doc(models::Route {
        title: "New Bot Token",
        method: "DELETE",
        path: "/bots/{id}/token",
        description: r#"
'Deletes' a bot token and reissues a new bot token. Use this if your bots
token ever gets leaked.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Bot],
    });

    // New User Token
    docs += &doc(models::Route {
        title: "New User Token",
        method: "DELETE",
        path: "/users/{id}/token",
        description: r#"
'Deletes' a user token and reissues a new user token. Use this if your user
token ever gets leaked.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    // New Server Token
    docs += &doc(models::Route {
        title: "New Server Token",
        method: "DELETE",
        path: "/servers/{id}/token",
        description: r#"
'Deletes' a server token and reissues a new server token. Use this if your server
token ever gets leaked.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Server],
    });

    docs += &doc_category("Bot Actions");

    docs += &doc(models::Route {
        title: "New Bot",
        method: "POST",
        path: "/users/{id}/bots",
        description: r#"
Creates a new bot. 

Set ``created_at``, ``last_stats_post`` to sometime in the past

Set ``api_token``, ``guild_count`` etc (unknown/not editable fields) to any 
random value of the same type

With regards to ``extra_owners``, put all of them as a ``BotOwner`` object
containing ``main`` set to ``false`` and ``user`` as a dummy ``user`` object 
containing ``id`` filled in and the rest of a ``user``empty strings. Set ``bot``
to false.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Bot::default(),
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Edit Bot",
        method: "PATCH",
        path: "/users/{id}/bots",
        description: r#"
Edits a existing bot. 

Set ``created_at``, ``last_stats_post`` to sometime in the past

Set ``api_token``, ``guild_count`` etc (unknown/not editable fields) to any 
random value of the same type

With regards to ``extra_owners``, put all of them as a ``BotOwner`` object
containing ``main`` set to ``false`` and ``user`` as a dummy ``user`` object 
containing ``id`` filled in and the rest of a ``user``empty strings. Set ``bot``
to false.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Bot::default(),
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Transfer Ownership",
        method: "PATCH",
        path: "/users/{user_id}/bots/{bot_id}/main-owner",
        description: r#"
Transfers bot ownership.

You **must** be main owner to use this endpoint.
"#,
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::Empty {},
        request_body: &models::BotOwner {
            main: true,
            user: models::User {
                id: "id here".to_string(),
                username: "Leave blank".to_string(),
                disc: "Leave blank".to_string(),
                avatar: "Leave blank".to_string(),
                status: models::Status::Unknown,
                bot: false,
            },
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Delete Bot",
        method: "DELETE",
        path: "/users/{user_id}/bots/{bot_id}",
        description: r#"
Deletes a bot.

You **must** be main owner to use this endpoint.
"#,
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Get Import Sources",
        method: "GET",
        path: "/import-sources",
        description: r#"
Returns a array of sources that a bot can be imported from.
"#,
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::ImportSourceList {
            sources: vec![
                models::ImportSourceListItem {
                    id: models::ImportSource::Rdl,
                    name: "Rovel Bot List".to_string()
                }
            ]
        },
        auth_types: vec![],
    });

    docs += &doc(models::Route {
        title: "Import Bot",
        method: "POST",
        path: "/users/{user_id}/bots/{bot_id}/import?src={source}",
        description: r#"
Imports a bot from a source listed in ``Get Import Sources``.
"#,
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::ImportQuery {
            src: models::ImportSource::Rdl,
            custom_source: Some("".to_string()),
        },
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Appeal");

    // New Appeal
    docs += &doc(models::Route {
        title: "New Appeal",
        method: "POST",
        path: "/users/{user_id}/bots/{bot_id}/appeal",
        description: r#"
Creates a appeal/request for a bot.

``request_type`` is a [AppealType](https://lynx.fateslist.xyz/docs/enums-ref#appealtype)

**Ideally only useful for custom clients**
"#,
        path_params: &models::GetUserBotPath {
            user_id: 0,
            bot_id: 0,
        },
        query_params: &models::Empty {},
        request_body: &models::Appeal {
            request_type: models::AppealType::Appeal,
            appeal: "This bot deserves to be unbanned because...".to_string(),
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Packs");

    docs += &doc(models::Route {
        title: "Add Pack",
        method: "GET",
        path: "/users/{id}/packs",
        description: r#"
Creates a bot pack. 

- Set ``id`` to empty string, 
- Set ``created_at`` to any datetime
- In user and bot, only ``id`` must be filled, all others can be left empty string
but must exist in the object
"#,
        path_params: &models::FetchBotPath { 
            id: 0
        },
        query_params: &models::Empty {},
        request_body: &models::BotPack::default(),
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Users");

    docs += &doc(models::Route {
        title: "Get Profile",
        method: "GET",
        path: "/profiles/{id}",
        description: r#"
Gets a user profile.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::Profile {
            vote_reminder_channel: Some("939123825885474898".to_string()),
            action_logs: vec![models::ActionLog::default()],
            ..models::Profile::default()
        },
        auth_types: vec![],
    });

    docs += &doc(models::Route {
        title: "Update Profile",
        method: "PATCH",
        path: "/profiles/{id}",
        description: r#"
Edits a user profile.

``user`` can be completely empty valued but the keys present in a User must
be present
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Profile {
            vote_reminder_channel: Some("939123825885474898".to_string()),
            action_logs: vec![models::ActionLog::default()],
            ..models::Profile::default()
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Put User Roles",
        method: "PUT",
        path: "/profiles/{id}/roles",
        description: r#"
Gives user roles on the Fates List support server
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::RoleUpdate::default(),
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Test Experiments",
        method: "GET",
        path: "/profiles/{id}/test-experiments",
        description: r#"
Internal route to showcase experiments
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::Empty {},
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Reviews");

    docs += &doc(models::Route {
        title: "Get Reviews",
        method: "GET",
        path: "/reviews/{id}",
        description: r#"
Gets reviews for a reviewable entity.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)

This reviewable entities id which is a ``i64`` is the id that is specifed in the
path.

``page`` must be greater than 0 or omitted (which will default to page 1).

``user_id`` is optional for this endpoint but specifying it will provide ``user_reviews`` if
the user has made a review. This will tell you the users review for the entity.

``per_page`` (amount of root/non-reply reviews per page) is currently set to 9. 
This may change in the future and is given by ``per_page`` key.

``from`` contains the index/count of the first review of the page.
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::ReviewQuery {
            page: Some(1),
            user_id: Some(0),
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Empty {},
        response_body: &models::ParsedReview {
            reviews: vec![models::Review::default()],
            user_review: Some(models::Review::default()),
            per_page: 9,
            from: 0,
            stats: models::ReviewStats {
                total: 78,
                average_stars: bigdecimal::BigDecimal::from_f32(8.8).unwrap(),
            },
        },
        auth_types: vec![],
    });

    docs += &doc(models::Route {
        title: "Create Review",
        method: "POST",
        path: "/reviews/{id}",
        description: r#"
Creates a review.

``id`` and ``page`` should be set to null or omitted though are ignored by this endpoint
so there should not be an error even if provided.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

The ``parent_id`` is optional and is used to create a reply to a review.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)

``review`` is a [Review](https://lynx.fateslist.xyz/docs/models-ref#review)

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::ReviewQuery {
            page: None,
            user_id: Some(0),
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Review {
            parent_id: Some(uuid::Uuid::new_v4()),
            ..models::Review::default()
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Edit Review",
        method: "PATCH",
        path: "/reviews/{id}",
        description: r#"
Edits a review.

``page`` should be set to null or omitted though are ignored by this endpoint
so there should not be an error even if provided.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)

This reviewable entities id which is a ``i64`` is the id that is specifed in the
path.

The id of the review must be specified as ``id`` in the request body which accepts a ``Review``
object. The ``user_id`` specified must *own*/have created the review being editted. Staff should
edit reviews using Lynx when required.

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::ReviewQuery {
            page: None,
            user_id: Some(0),
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Review {
            id: Some(uuid::Uuid::new_v4()),
            ..models::Review::default()
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Delete Review",
        method: "DELETE",
        path: "/reviews/{rid}",
        description: r#"
Deletes a review

``rid`` must be a valid uuid.

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header. ``page`` is currently ignored

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)

``target_type`` is not currently checked but it is a good idea to set it anyways. You must
set this anyways so you might as well set it correctly.
"#,
        path_params: &models::ReviewDeletePath {
            rid: uuid::Uuid::new_v4().to_hyphenated().to_string(),
        },
        query_params: &models::ReviewQuery {
            page: None,
            user_id: Some(0),
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc(models::Route {
        title: "Vote Review",
        method: "PATCH",
        path: "/reviews/{rid}/votes",
        description: r#"
Creates a vote for a review

``rid`` must be a valid uuid.

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header. 

**Unlike other review APIs, ``user_id`` here is in request body as ReviewVote object**

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)

**This endpoint does not require ``target_type`` at all. You can safely omit it**
"#,
        path_params: &models::ReviewDeletePath {
            rid: uuid::Uuid::new_v4().to_hyphenated().to_string(),
        },
        query_params: &models::Empty {},
        request_body: &models::ReviewVote {
            user_id: "user id here".to_string(),
            upvote: true,
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::User],
    });

    docs += &doc_category("Stats");

    docs += &doc(models::Route {
        title: "Get List Stats",
        method: "GET",
        path: "/stats",
        description: r#"
Returns the bot list stats. This currently returns the full list of all bots
as a vector/list of IndexBot structs.

As a client, it is your responsibility, to parse this. Pagination may be added
if the list grows and then requires it.
"#,
        path_params: &models::Empty {},
        query_params: &models::Empty {},
        request_body: &models::Empty {},
        response_body: &models::ListStats {
            bots: index_bots.clone(),
            ..models::ListStats::default()
        },
        auth_types: vec![],
    });

    docs += &doc_category("Resources");

    docs += &doc(models::Route {
        title: "Create Resource",
        method: "POST",
        path: "/resources/{id}",
        description: r#"
Creates a resource. Both bots and servers support these however only bots 
support the frontend resource creator in Bot Settings as of right now.

The ``id`` here must be the resource id

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::TargetQuery {
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Resource::default(),
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Bot, models::RouteAuthType::Server],
    });

    docs += &doc(models::Route {
        title: "Delete Resource",
        method: "DELETE",
        path: "/resources/{id}",
        description: r#"
Deletes a resource. Both bots and servers support these however only bots 
support the frontend resource creator in Bot Settings as of right now.

The ``id`` here must be the resource id

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::ResourceDeleteQuery {
            id: uuid::Uuid::new_v4().to_hyphenated().to_string(),
            target_type: models::TargetType::Bot,
        },
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Bot, models::RouteAuthType::Server],
    });

    docs += &doc_category("Commands");

    docs += &doc(models::Route {
        title: "Create Bot Command",
        method: "POST",
        path: "/bots/{id}/commands",
        description: r#"
Creates a command.

The ``id`` here must be the bot id you wish to add the command for

**This performs a *upsert* meaning it will either create or update 
the command depending on its ``name``.**

**Only post up to 10-20 commands at a time, otherwise requests may be truncated
or otherwise fail with odd errors.  If you have more than this, then perform 
multiple requests**

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/enums-ref#targettype)
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::TargetQuery {
            target_type: models::TargetType::Bot,
        },
        request_body: &models::BotCommandVec {
            commands: vec![models::BotCommand::default()],
        },
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Bot],
    });

    docs += &doc(models::Route {
        title: "Delete Bot Command",
        method: "DELETE",
        path: "/bots/{id}/commands",
        description: r#"
DELETE a command.

The ``id`` here must be the bot id you wish to add the command for

``names`` and ``ids`` must be a ``|`` seperated list of ``names`` or valid
UUIDs in the case of ids. Bad names/ids will be ignored
"#,
        path_params: &models::FetchBotPath { id: 0 },
        query_params: &models::CommandDeleteQuery {
            nuke: Some(false),
            names: Some("command name|command name 2".to_string()),
            ids: Some("id 1|id 2".to_string()),
        },
        request_body: &models::Empty {},
        response_body: &models::APIResponse {
            done: true,
            reason: None,
            context: None,
        },
        auth_types: vec![models::RouteAuthType::Bot],
    });

    // Return docs
    docs
}
