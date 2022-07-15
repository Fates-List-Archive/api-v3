use crate::models;
use bigdecimal::FromPrimitive;
use serde::Serialize;
use std::fmt::Debug;
use strum::IntoEnumIterator;
use serde_json::json;
use log::debug;
use std::io::{Write as W2};
use std::fmt::Write;

const PATH_PARAMS: &str = "Path Parameters";
const QUERY_PARAMS: &str = "Query Parameters";
const REQ_BODY: &str = "Request Body";
const RESP_BODY: &str = "Response Body";


fn body<T: Serialize>(typ: &str, obj: T) -> String {
    if typ == PATH_PARAMS || typ == QUERY_PARAMS {
        return format!(
            "**{typ}**\n\n{body_desc}\n\n",
            body_desc = docser::serialize_docs(&obj).unwrap(),
        )    
    }

    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);

    obj.serialize(&mut ser).unwrap();

    format!(
        // If this is ever editted, make sure to update the panic call accordingly
        "**{typ}**\n\n{body_desc}\n\n**{typ} Example**\n\n```json\n{body_ex}\n```\n\n",
        body_desc = docser::serialize_docs(&obj).unwrap(),
        body_ex = String::from_utf8(ser.into_inner()).unwrap()
    )
}

struct Sanity {
    id: String,
    method: String
}

fn doc(
    basic_api: &str,
    routes: Vec<models::RouteList>
) {
    for route in routes {
        debug!("Creating new doc file for: {}", route.file_name);

        match std::env::var_os("SANITY") {
            None => {}, 
            Some(_) => {
                // Check length to ensure all routes
                let contents = std::fs::read_to_string("src/main.rs")
                    .expect("Something went wrong reading src/main.rs to validate docs");
    
                for line in contents.lines() {
                    let check_d = line.replace(' ', "").replace('\t', "");
                    if check_d.starts_with(".service") {
                        // If we find a service, we parse it and remove comments and then split by namespace
                        let service = check_d.replace(".service(", "");
                        let service = service.split(')').collect::<Vec<&str>>();
                        let service = service.get(0).unwrap();
                        let service = service.split("::").collect::<Vec<&str>>();
                        
                        let route_service = route.file_name.replace(".md", "").replace('-', "");

                        if *(*service.get(0).unwrap()) != route_service {
                            continue;
                        }

                        debug!("Found service {:?}", service);
                        
                        // This is highly inefficient but on most devices, hits cache and as such is not a problem
                        let router_file = std::fs::read_to_string(format!("src/{}.rs", route_service))
                            .expect("Something went wrong reading src/main.rs to validate docs");

                        // Validate that we have the service
                        let endpoint = service.get(1).unwrap();

                        // Step 1. Does endpoint exist in route list?

                        let mut got_ep = false;
                        for doc in &route.routes {
                            if doc.title.replace(' ', "_").to_lowercase() == **endpoint {
                                got_ep = true;
                                break;
                            }
                        }

                        assert!(got_ep, "Could not find endpoint {} in route list", endpoint);

                        // Find the endpoint in router_file
                        let lines = router_file.lines();
                        let mut peekable = lines.peekable();
                        
                        let mut found_services = Vec::new();

                        while let Some(line) = peekable.next() {
                            if line.starts_with("#[") {
                                let next_line = peekable.peek().expect("Could not peek next line of file");
                                
                                // Appeasing the borrow checker is fun
                                let service = next_line
                                    .replace("async fn", "")
                                    .replace("pub", "")
                                    .replace(' ', "");
                                    
                                let service = service.split('(');

                                let service = service
                                    .collect::<Vec<&str>>();
                                
                                let service = service
                                    .get(0)
                                    .unwrap();

                                let method = line.split('(');

                                let method = method
                                    .collect::<Vec<&str>>();
                                
                                let method = method
                                    .get(0)
                                    .unwrap();
                                
                                let method = method
                                    .replace("#[", "");
                                
                                debug!("Found service (in file) {:?} with method {:?}", service, method);

                                found_services.push(Sanity {
                                    id: (**service).to_string(),
                                    method,
                                });
                            }
                        }

                        // Step 2. Does all endpoint in docs exist in found_services with the proper method?
                        for doc in &route.routes { 
                            let ep = doc.title.replace(' ', "_").to_lowercase();

                            let mut found_ep = false;
                            for service in &found_services {
                                if service.id == ep {
                                    assert!(service.method.to_uppercase() == doc.method, "Found endpoint {} in found_services but got invalid method {}, expected {}", ep, service.method, doc.method);
                                    found_ep = true;
                                    break;
                                }
                            }

                            assert!(found_ep, "Could not find endpoint {} in found_services", ep);
                        }
                    }
                }
            },
        };

        new_doc_file(basic_api.to_string(), route);
    }
}

#[inline]
// Given a type, return what starts_with should be called with
fn typ_to_swith(typ: &str) -> String {
    "**".to_string() + typ + "**"
}

fn new_doc_file(
    basic_api: String,
    routes: models::RouteList,
) {
    let mut docs = vec![basic_api + "\n"];

    for route in routes.routes {
        let mut auth_needed: String = "".to_string();
        let mut i = 1;
        let auth_lengths = route.auth_types.clone().len();
        for auth in route.auth_types {
            if auth == models::RouteAuthType::Bot {
                auth_needed += "[Bot](#authorization)";
                if i < auth_lengths {
                    auth_needed += ", ";
                }
            } else if auth == models::RouteAuthType::User {
                auth_needed += "[User](#authorization)";
                if i < auth_lengths {
                    auth_needed += ", ";
                }
            } else if auth == models::RouteAuthType::Server {
                auth_needed += "[Server](#authorization)";
                if i < auth_lengths {
                    auth_needed += ", ";
                }
            } else if auth == models::RouteAuthType::Special {
                auth_needed += "[Special](#authorization)";
                if i < auth_lengths {
                    auth_needed += ", ";
                }
            }
            i += 1;
        } 

        if auth_needed.is_empty() {
            auth_needed = "None".to_string();
        }

        // Some basic checks to make sure we don't have any errors
        assert!(route.path_params.is_empty() || route.path_params.starts_with(&typ_to_swith(PATH_PARAMS)), "{} is not a valid route: path_params is not of type PATH_PARAMS", route.title);

        assert!(route.query_params.is_empty() || route.query_params.starts_with(&typ_to_swith(QUERY_PARAMS)), "{} is not a valid route: query_params is not of type QUERY_PARAMS", route.title);

        assert!(route.request_body.is_empty() || route.request_body.starts_with(&typ_to_swith(REQ_BODY)), "{} is not a valid route: request_body is not of type REQ_BODY", route.title);

        assert!(route.response_body.is_empty() || route.response_body.starts_with(&typ_to_swith(RESP_BODY)), "{} is not a valid route: response_body is not of type RESP_BODY", route.title);

        docs.push(
            format!(
                "## {title}\n### {method} {url}\n{desc}\n{query_params}\n{path_params}\n{request_body}\n{response_body}\n**Authorization Needed** | {auth_needed}\n\n\n",
                title = route.title,
                method = route.method,
                url = "`https://api.fateslist.xyz`".to_string() + route.path,
                desc = route.description,
                request_body = route.request_body,
                response_body = route.response_body,
                query_params = route.query_params,
                path_params = route.path_params,
                auth_needed = auth_needed
            )
        );
    }

    let path = match std::env::var_os("HOME") {
        None => {
            panic!("$HOME not set");
        }
        Some(path) => std::path::PathBuf::from(path),
    };

    let endpoints_dir = path.into_os_string().into_string().unwrap() + "/electrodragon/api-docs/endpoints";

    std::fs::create_dir_all(&endpoints_dir).expect("Unable to create directory for endpoint docs");

    let mut file = std::fs::File::create(endpoints_dir + "/" + routes.file_name).unwrap();

    file.write_all(docs.join("").as_bytes()).unwrap();
}

// This is a doc file. Lots of lines are ok
#[allow(clippy::too_many_lines)]
pub fn document_routes() {
    const BASIC_API: &str = r#"
**API URL**: ``https://api.fateslist.xyz``

**Widgets Documentation:** ``https://lynx.fateslist.xyz/widgets`` (docs for widgets available at https://lynx.fateslist.xyz/widgets)

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
you prefix the token with `User`. **A access token (for custom clients)
can also be used on *most* endpoints as long as the token is prefixed with 
``Frostpaw``**

- **Special:** These endpoint employ their own authentication system (such as ``slwebset``)

## Base Response

A default API Response will be of the below format:

```json
{
    done: false | true,
    reason: "" | null,
    context: "" | null
}
```
"#;

    let index_bots = vec![models::IndexBot::default()];

    let tags = vec![models::Tag::default()];

    let features = vec![models::Feature::default()];        

    // TODO: For each route, add doc system
    doc(
        BASIC_API,
        vec![
            models::RouteList {
                file_name: "core.md",
                routes: vec![
                    models::Route {
                        title: "Index",
                        method: "GET",
                        path: "/index",
                        path_params: "",
                        query_params: &body(QUERY_PARAMS, &models::IndexQuery {
                            target_type: models::TargetType::Server,
                        }),
                        description: "Returns the index for bots and servers",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Index {
			    random: models::IndexBot::default(),
                            top_voted: index_bots.clone(),
                            certified: index_bots.clone(),
                            new: index_bots.clone(),
                            tags: tags.clone(),
                            features: features.clone(),
                        }),
                        auth_types: vec![]
                    },


                    models::Route {
                        title: "Ping",
                        method: "GET",
                        path: "/ping",
                        path_params: "",
                        query_params: "",
                        description: "Returns nothing",
                        request_body: "",
                        response_body: "",
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Set Server Listing By Web",
                        method: "POST",
                        path: "/slwebset",
                        path_params: "",
                        query_params: "",
                        description: "Sets the server listing on the web (after ``/webset`` command). Set ``Authorization`` to the token given by webset",
                        request_body: &body(REQ_BODY, &models::SlwebsetJson::default()),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: Some("https://discord.com/.........".to_string()),
                        }),
                        auth_types: vec![models::RouteAuthType::Special]
                    },

                    models::Route {
                        title: "Get Experiment List",
                        method: "GET",
                        path: "/experiments",
                        path_params: "",
                        query_params: "",
                        description: "Returns all currently available experiments",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::ExperimentList {
                            user_experiments: vec![models::UserExperimentListItem {
                                name: models::UserExperiments::Unknown.to_string(),
                                value: models::UserExperiments::Unknown,
                            }],
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Resolve Vanity",
                        method: "GET",
                        path: "/code/{code}",
                        path_params: &body(PATH_PARAMS, &models::VanityPath {
                            code: "my-vanity".to_string(),
                        }),
                        query_params: "",
                        description: "Resolves the vanity for a bot/server in the list",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Vanity {
                            target_id: "0000000000".to_string(),
                            target_type: "bot | server".to_string(),
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Get Partners",
                        method: "GET",
                        path: "/partners",
                        path_params: "",
                        query_params: "",
                        description: "Get current partnership list",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Partners::default()),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Search List",
                        method: "GET",
                        path: "/search?q={query}",
                        path_params: "",
                        query_params: &body(QUERY_PARAMS, &models::SearchQuery {
                            q: "mew".to_string(),
                            gc_from: 1,
                            gc_to: -1,
                        }),
                        description: r#"
Searches the list based on a query named ``q``. 
        
Using -1 for ``gc_to`` will disable ``gc_to`` field"#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Search {
                            bots: vec![models::IndexBot::default()],
                            servers: vec![models::IndexBot::default()],
                            packs: vec![models::BotPack::default()],
                            profiles: vec![models::SearchProfile::default()],
                            tags: models::SearchTags {
                                bots: vec![models::Tag::default()],
                                servers: vec![models::Tag::default()]
                            },
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Search Tags",
                        method: "GET",
                        path: "/search-tags?q={query}",
                        path_params: "",
                        query_params: &body(QUERY_PARAMS, &models::SearchTagQuery {
                            q: "mew".to_string(),
                        }),
                        description: "Searches the list based on a tag named ``q``.",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Search {
                            bots: vec![models::IndexBot::default()],
                            servers: vec![models::IndexBot::default()],
                            packs: vec![models::BotPack::default()],
                            profiles: vec![models::SearchProfile::default()],
                            tags: models::SearchTags {
                                bots: vec![models::Tag::default()],
                                servers: vec![models::Tag::default()]
                            },
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Mini Index",
                        method: "GET",
                        path: "/mini-index",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Index {
			    random: models::IndexBot::default(),
                            new: Vec::new(),
                            top_voted: Vec::new(),
                            certified: Vec::new(),
                            tags: tags.clone(),
                            features: features.clone(),
                        }),
                        description: r#"
Returns a mini-index which is basically a Index but with only ``tags``
and ``features`` having any data. Other fields are empty arrays/vectors.

This is used internally by sunbeam for the add bot system where a full bot
index is too costly and making a new struct is unnecessary.
                "#,
                        auth_types: vec![],
                    }
                ],
            },

            models::RouteList {
                file_name: "auth.md",
                routes: vec![

                    models::Route {
                        title: "Get OAuth2 Link",
                        method: "GET",
                        path: "/oauth2",
                        description: r#"
Returns the oauth2 link used to login with. ``reason`` contains the state UUID

- `Frostpaw-Server` header must be set to `https://fateslist.xyz` if you are a custom client
- If you are a custom client, then ignore the state present here and instead set `state` to `Bayshine.${YOUR CLIENT ID}.${CURRENT TIME}.${HMAC PAYLOAD}` where 
client ID is the client ID given during whitelisting, CURRENT TIME is the current time in Unix Epoch and HMAC PAYLOAD is that same current time HMAC-SHA256
signed with your client secret given to you during whitelisting. **You must calculate state server side**

Once login succeeds and is authorized by the user, then the user will be redirected to ${YOUR DOMAIN}/frostpaw?data=${BASE64 encoded OauthUserLogin}
                "#,
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: Some("https://discord.com/.........".to_string()),
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Get Frostpaw Client",
                        method: "GET",
                        path: "/frostpaw/clients/{id}",
                        description: r#"
Returns the Frostpaw client with the given ID.
                        "#,
                        path_params: &body(PATH_PARAMS, &models::StringIDPath {
                            id: "client id here".to_string(),
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::FrostpawClient::default()),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Refresh Frostpaw Token",
                        method: "POST",
                        path: "/frostpaw/clients/{client_id}/refresh",
                        description: r#"
Refreshes a token for the given client.
                        "#,
                        path_params: &body(PATH_PARAMS, &models::StringIDPath {
                            id: "client id here".to_string(),
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: Some("Error code, if any".to_string()),
                            context: Some("Refresh token, if everything went ok :)".to_string()),
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Create OAuth2 Login",
                        method: "POST",
                        path: "/oauth2",
                        description: r#"
Creates a oauth2 login given a code. 

**This API (as well as the below) is already done for custom clients by the actual site**

- Set `frostpaw` in the JSON if you are a custom client
- `Frostpaw-Server` header must be set to `https://fateslist.xyz`
- ``frostpaw_blood`` (client ID), ``frostpaw_claw`` (hmac'd time you sent) and 
``frostpaw_claw_unseathe_time`` (time you sent in state) are internal fields used 
by the site to login.
                "#,
                        path_params: "",
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::OauthDoQuery {
                            code: "code from discord oauth".to_string(),
                            state: Some("Random UUID right now".to_string()),
                            frostpaw: true,
                            frostpaw_blood: None,
                            frostpaw_claw: None,
                            frostpaw_claw_unseathe_time: None
                        }),
                        response_body: &body(RESP_BODY, &models::OauthUserLogin::default()),
                        auth_types: vec![]
                    },
                ] 
            },

            models::RouteList {
                file_name: "security.md",
                routes: vec![
                    models::Route {
                        title: "New Bot Token",
                        method: "DELETE",
                        path: "/bots/{id}/token",
                        description: r#"
'Deletes' a bot token and reissues a new bot token. Use this if your bots
token ever gets leaked! Also used by the official client"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::Bot],
                    },

                    models::Route {
                        title: "New User Token",
                        method: "DELETE",
                        path: "/users/{id}/token",
                        description: r#"
'Deletes' a user token and reissues a new user token. Use this if your bots
token ever gets leaked! Also used by the official client"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "New Server Token",
                        method: "DELETE",
                        path: "/servers/{id}/token",
                        description: r#"
'Deletes' a server token and reissues a new server token. Use this if your server
token ever gets leaked."#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { 
                            id: 0 
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::Server],
                    },

                    models::Route {
                        title: "Revoke Frostpaw Client Auth",
                        method: "DELETE",
                        path: "/users/{id}/frostpaw/clients/{client_id}",
                        description: r#"
'Deletes' a user token and reissues a new user token. Use this if your user
token ever gets leaked.
                "#,
                        path_params: &body(PATH_PARAMS, &models::UserClientAuth { 
                            id: 0, 
                            client_id: "client_id".to_string() 
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                ]
            },

            models::RouteList {
                file_name: "bot-actions.md",
                routes: vec![
                    models::Route {
                        title: "Post Stats",
                        method: "POST",
                        path: "/bots/{id}/stats",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath {
                            id: 0,
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::BotStats {
                            guild_count: 3939,
                            shard_count: Some(48484),
                            shards: Some(vec![149, 22020]),
                            user_count: Some(39393),
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse::default()),
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
```"#,
                        auth_types: vec![models::RouteAuthType::Bot]
                    },

                    models::Route {
                        title: "Get Bot",
                        method: "GET",
                        path: "/bots/{id}",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath::default()),
                        query_params: "",
                        description: r#"
Fetches bot information given a bot ID. If not found, 404 will be returned. 

This endpoint handles both bot IDs and client IDs

- ``long_description/css`` is sanitized with ammonia by default, use `long_description_raw` if you want the unsanitized version
- All responses are cached for a short period of time. There is *no* way to opt out at this time
- Some fields have been renamed or removed from API v2 (such as ``promos`` which may be readded at a later date)

This API returns some empty fields such as ``webhook``, ``webhook_secret``, ``api_token`` and more. 
This is to allow reuse of the Bot struct in Get Bot Settings which *does* contain this sensitive data. 

**Set the Frostpaw header if you are a custom client. Send Frostpaw-Invite header on invites**"#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Bot::default()),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Get Bot Settings",
                        method: "GET",
                        path: "/users/{user_id}/bots/{bot_id}/settings",
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::BotSettings {
                            bot: models::Bot::default(),
                            context: models::BotSettingsContext { tags, features },
                        }),
                        description: r#"
Returns the bot settings.

The ``bot`` here is equivalent to a Get Bot response with the following
differences:

- Sensitive fields (see examples) like ``webhook``, ``api_token``, 
``webhook_secret`` and others are filled out here
- This API only allows bot owners (not even staff) to use it, otherwise it will 400!

Staff members *must* instead use Lynx."#,
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Random Bot",
                        method: "GET",
                        path: "/random-bot",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::IndexBot::default()),
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
```"#,
                        auth_types: vec![]
                    },
                    
                    models::Route {
                        title: "Add Bot",
                        method: "POST",
                        path: "/users/{id}/bots",
                        description: r#"
Creates a new bot. 

Set ``created_at``, ``last_stats_post`` to sometime in the past

Set ``api_token``, ``guild_count`` etc (unknown/not editable fields) to any 
random value of the same type.

With regards to ``extra_owners``, put all of them as a ``BotOwner`` object
containing ``main`` set to ``false`` and ``user`` as a dummy ``user`` object 
containing ``id`` filled in and the rest of a ``user``empty strings. Set ``bot``
to false."#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::Bot::default()),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
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
to false."#,
                        path_params: &body(PATH_PARAMS, models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::Bot::default()),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Transfer Ownership",
                        method: "PATCH",
                        path: "/users/{user_id}/bots/{bot_id}/main-owner",
                        description: r#"
Transfers bot ownership.

You **must** be main owner of the bot to use this endpoint.
                "#,
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::BotOwner {
                            main: true,
                            user: models::User {
                                id: "id here".to_string(),
                                username: "Leave blank".to_string(),
                                disc: "Leave blank".to_string(),
                                avatar: "Leave blank".to_string(),
                                status: models::Status::Unknown,
                                bot: false,
                            },
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Delete Bot",
                        method: "DELETE",
                        path: "/users/{user_id}/bots/{bot_id}",
                        description: r#"
Deletes a bot.

You **must** be main owner of the bot to use this endpoint."#,
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Get Import Sources",
                        method: "GET",
                        path: "/import-sources",
                        description: r"Returns a array of sources that a bot can be imported from.",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::ImportSourceList {
                            sources: vec![
                                models::ImportSourceListItem {
                                    id: models::ImportSource::Rdl,
                                    name: "Rovel Bot List".to_string()
                                }
                            ]
                        }),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Import Bot",
                        method: "POST",
                        path: "/users/{user_id}/bots/{bot_id}/import?src={source}",
                        description: "Imports a bot from a source listed in ``Get Import Sources``",
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: &body(QUERY_PARAMS, &models::ImportQuery {
                            src: models::ImportSource::Rdl,
                            custom_source: Some("".to_string()),
                        }),
                        request_body: &body(REQ_BODY, &models::ImportBody {
                            ext_data: Some(std::collections::HashMap::from([("key".to_string(), json!("value"))]))
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            },

            models::RouteList {
                file_name: "server-actions.md",
                routes: vec![

                    models::Route {
                        title: "Get Server",
                        method: "GET",
                        path: "/servers/{id}",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath::default()),
                        query_params: "",
                        description: r#"
Fetches server information given a server/guild ID. If not found, 404 will be returned. 

- *``long_description/css`` is sanitized with ammonia by default, use `long_description_raw` if you want the unsanitized version*
- All responses are cached for a short period of time. There is *no* way to opt out unlike API v2
- Some fields have been renamed or removed
- ``invite_link`` is returned, however is always None unless ``Frostpaw-Invite`` header is set which then pushes you into 
server privacy restrictions. **Note that when fetching invite links, requires login to join is now enabled by default for all new servers**
                
**Set the Frostpaw header if you are a custom client. This is also needed for invites to work**"#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Server::default()),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Random Server",
                        method: "GET",
                        path: "/random-server",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::IndexBot::default()),
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
```"#,
                        auth_types: vec![]
                    }                    
                ]
            },

            models::RouteList {
                file_name: "votes.md",
                routes: vec![

                    models::Route {
                        title: "Create Bot Vote",
                        method: "PATCH",
                        path: "/users/{user_id}/bots/{bot_id}/votes",
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: &body(QUERY_PARAMS, &models::VoteBotQuery { test: true }),
                        description: "This endpoint creates a vote for a bot which can only be done *once* every 8 hours.",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: false,
                            reason: Some("Error code".to_string()),
                            context: Some("Some context on the error".to_string()),
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Create Server Vote",
                        method: "PATCH",
                        path: "/users/{user_id}/servers/{server_id}/votes",
                        path_params: &body(PATH_PARAMS, &models::GetUserServerPath {
                            user_id: 0,
                            server_id: 0,
                        }),
                        query_params: &body(QUERY_PARAMS, &models::VoteBotQuery { test: true }),
                        description: r#"
This endpoint creates a vote for a server which can only be done *once* every 8 hours
and is independent from a bot vote.
                    "#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: false,
                            reason: Some("Why the vote failed or any extra info to send to client if the vote succeeded".to_string()),
                            context: Some("Some context on the vote".to_string()),
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Get Bot Votes",
                        method: "GET",
                        path: "/users/{user_id}/bots/{bot_id}/votes",
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: "",
                        description: r#"
Endpoint to check amount of votes a user has.

- votes | The amount of votes the bot has.
- voted | Whether or not the user has *ever* voted for a bot in the past 8 hours.
- timestamps | A list of timestamps that the user has voted for the bot on that has been recorded.
- expiry | The time when the user can next vote.
- vote_right_now | Whether a user can vote right now. Currently equivalent to `vote_epoch < 0`.

- Unlike API v2, this *does not* require authorization to use. This is to speed up responses and 
because the last thing people want to scrape are Fates List user votes anyways. **You should not rely on
this however, it is prone to change *anytime* in the future and may return bogus results for privacy purposes**.
- ``vts`` has been renamed to ``timestamps``

**This endpoint will return bogus data if "Hide votes to other users" is enabled**"#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::UserVoted {
                            votes: 10,
                            voted: true,
                            expiry: 101,
                            timestamps: vec![chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc)],
                            vote_right_now: false,
                        }),
                        auth_types: vec![]
                    },

                    models::Route {
                        title: "Get Server Votes",
                        method: "GET",
                        path: "/users/{user_id}/servers/{server_id}/votes",
                        path_params: &body(PATH_PARAMS, &models::GetUserServerPath {
                            user_id: 0,
                            server_id: 0,
                        }),
                        query_params: "",
                        description: r#"
Endpoint to check amount of votes a user has.

- votes | The amount of votes the server has.
- voted | Whether or not the user has *ever* voted for a server in the past 8 hours.
- timestamps | A list of timestamps that the user has voted for the server on that has been recorded.
- expiry | The time when the user can next vote.
- vote_right_now | Whether a user can vote right now. Currently equivalent to `vote_epoch < 0`.
                
- Unlike API v2, this does not require authorization to use. This is to speed up responses and 
because the last thing people want to scrape are Fates List user votes anyways. **You should not rely on
this however, it is prone to change *anytime* in the future and may return bogus results for privacy purposes**.
- ``vts`` has been renamed to ``timestamps``

**This endpoint will return bogus data if "Hide votes to other users" is enabled**"#,
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::UserVoted {
                            votes: 10,
                            voted: true,
                            expiry: 101,
                            timestamps: vec![chrono::DateTime::<chrono::Utc>::from_utc(chrono::NaiveDateTime::from_timestamp(0, 0), chrono::Utc)],
                            vote_right_now: false,
                        }),
                        auth_types: vec![]
                    }
                ]
            },

            models::RouteList {
                file_name: "appeals.md",
                routes: vec![

                    models::Route {
                        title: "Create Bot Appeal",
                        method: "POST",
                        path: "/users/{user_id}/bots/{bot_id}/appeal",
                        description: r#"
Creates a appeal for a bot.

``request_type`` is a [AppealType](./enums#appealtype)
                "#,
                        path_params: &body(PATH_PARAMS, &models::GetUserBotPath {
                            user_id: 0,
                            bot_id: 0,
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::Appeal {
                            request_type: models::AppealType::Appeal,
                            appeal: "This bot deserves to be unbanned because...".to_string(),
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Create Server Appeal",
                        method: "POST",
                        path: "/users/{user_id}/servers/{server_id}/appeal",
                        description: r#"
Creates a appeal for a server.

**Currently only `report` is supported by this endpoint**

``request_type`` is a [AppealType](./enums#appealtype)
                "#,
                        path_params: &body(PATH_PARAMS, &models::GetUserServerPath {
                            user_id: 0,
                            server_id: 0,
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::Appeal {
                            request_type: models::AppealType::Appeal,
                            appeal: "This server deserves to be unbanned because...".to_string(),
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            },

            models::RouteList {
                file_name: "packs.md",
                routes: vec![
                    models::Route {
                        title: "Add Pack",
                        method: "POST",
                        path: "/users/{id}/packs",
                        description: r#"
Creates a bot pack. 

- Set ``id`` to empty string, 
- Set ``created_at`` to any datetime
- In user and bot, only ``id`` must be filled, all others can be left empty string
but must exist in the object"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { 
                            id: 0
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::BotPack::default()),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Edit Pack",
                        method: "PATCH",
                        path: "/users/{id}/packs",
                        description: r#"
Edits a bot pack. 

- Set ``id`` to the pack id that is to be editted, 
- Set ``created_at`` to any datetime
- In user and bot, only ``id`` must be filled, all others can be left empty string
but must exist in the object"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { 
                            id: 0
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::BotPack {
                            id: uuid::Uuid::new_v4().to_string(),
                            ..models::BotPack::default()
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Delete Pack",
                        method: "DELETE",
                        path: "/users/{user_id}/packs/{pack_id}",
                        description: r#"
Deletes a bot pack. 

- Set ``pack_id`` to the pack id that is to be editted
- This endpoint may not always delete the pack in question in certain cases (pack not existant)"#,
                        path_params: &body(PATH_PARAMS, &models::GetUserPackPath { 
                            user_id: 0,
                            pack_id: uuid::Uuid::new_v4().to_string()
                        }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            },

            models::RouteList {
                file_name: "user.md",
                routes: vec![
                    models::Route {
                        title: "Get User From ID",
                        method: "GET",
                        path: "/blazefire/{id}",
                        description: r#"
Gets a User object given a ID. 

Internally is used by client for extra owner rendering etc.

May be used by our partners to get user information."#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::User::default()),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Get User Perms",
                        method: "GET",
                        path: "/baypaw/perms/{id}",
                        description: r#"
Gets the permissions of a user from Baypaw (our microservices handling user fetching and permissions)

Internally is used by client for extra owner rendering etc."#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &bristlefrost::models::StaffRole::default()),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Get Profile",
                        method: "GET",
                        path: "/profiles/{id}",
                        description: "Gets a user profile.",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::Profile {
                            vote_reminder_channel: Some("939123825885474898".to_string()),
                            action_logs: vec![models::ActionLog::default()],
                            connections: vec![models::FrostpawUserConnection::default()],
                            ..models::Profile::default()
                        }),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Update Profile",
                        method: "PATCH",
                        path: "/profiles/{id}",
                        description: r#"
Edits a user profile.

``user`` can be completely empty valued but the keys present in a User must
be present"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::Profile {
                            vote_reminder_channel: Some("939123825885474898".to_string()),
                            action_logs: vec![models::ActionLog::default()],
                            ..models::Profile::default()
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Receive Profile Roles",
                        method: "PUT",
                        path: "/profiles/{id}/roles",
                        description: "Gives user roles on the Fates List support server",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::RoleUpdate::default()),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            },

            models::RouteList {
                file_name: "reviews.md",
                routes: vec![

                    models::Route {
                        title: "Get Reviews",
                        method: "GET",
                        path: "/reviews/{id}",
                        description: r#"
Gets reviews for a reviewable entity.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/endpoints/enums#targettype)

This reviewable entities id which is a ``i64`` is the id that is specifed in the
path.

``page`` must be greater than 0 or omitted (which will default to page 1).

``user_id`` is optional for this endpoint but specifying it will provide ``user_reviews`` if
the user has made a review. This will tell you the users review for the entity.

``per_page`` (amount of root/non-reply reviews per page) is currently set to 9. 
This may change in the future and is given by ``per_page`` key.

``from`` contains the index/count of the first review of the page."#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::ReviewQuery {
                            page: Some(1),
                            user_id: Some(0),
                            target_type: models::TargetType::Bot,
                        }),
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::ParsedReview {
                            reviews: vec![models::Review::default()],
                            user_review: Some(models::Review::default()),
                            per_page: 9,
                            from: 0,
                            stats: models::ReviewStats {
                                total: 78,
                                average_stars: bigdecimal::BigDecimal::from_f32(8.8).unwrap(),
                            },
                        }),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Add Review",
                        method: "POST",
                        path: "/reviews/{id}",
                        description: r#"
Creates a review.

``id`` and ``page`` should be set to null or omitted though are ignored by this endpoint
so there should not be an error even if provided.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

The ``parent_id`` is optional and is used to create a reply to a review.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/endpoints/enums#targettype)

``review`` is a [Review](https://lynx.fateslist.xyz/docs/endpoints/enums#review)

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::ReviewQuery {
                            page: None,
                            user_id: Some(0),
                            target_type: models::TargetType::Bot,
                        }),
                        request_body: &body(REQ_BODY, &models::Review {
                            parent_id: Some(uuid::Uuid::new_v4()),
                            ..models::Review::default()
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Edit Review",
                        method: "PATCH",
                        path: "/reviews/{id}",
                        description: r#"
Edits a review.

``page`` should be set to null or omitted though are ignored by this endpoint
so there should not be an error even if provided.

A reviewable entity is currently only a bot or a server. Profile reviews are a possibility
in the future.

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/endpoints/enums#targettype)

This reviewable entities id which is a ``i64`` is the id that is specifed in the
path.

The id of the review must be specified as ``id`` in the request body which accepts a ``Review``
object. The ``user_id`` specified must *own*/have created the review being editted. Staff should
edit reviews using Lynx when required.

``user_id`` is *required* for this endpoint and must be the user making the review. It must
also match the user token sent in the ``Authorization`` header"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::ReviewQuery {
                            page: None,
                            user_id: Some(0),
                            target_type: models::TargetType::Bot,
                        }),
                        request_body: &body(REQ_BODY, &models::Review {
                            id: Some(uuid::Uuid::new_v4()),
                            ..models::Review::default()
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
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

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/endpoints/enums#targettype)

``target_type`` is not currently checked but it is a good idea to set it anyways. You must
set this anyways so you might as well set it correctly."#,
                        path_params: &body(PATH_PARAMS, &models::ReviewDeletePath {
                            rid: uuid::Uuid::new_v4().to_hyphenated().to_string(),
                        }),
                        query_params: &body(QUERY_PARAMS, &models::ReviewQuery {
                            page: None,
                            user_id: Some(0),
                            target_type: models::TargetType::Bot,
                        }),
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
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

``target_type`` is a [TargetType](https://lynx.fateslist.xyz/docs/endpoints/enums#targettype)

**This endpoint does not require ``target_type`` at all. You can safely omit it**
                "#,
                        path_params: &body(PATH_PARAMS, &models::ReviewDeletePath {
                            rid: uuid::Uuid::new_v4().to_hyphenated().to_string(),
                        }),
                        query_params: "",
                        request_body: &body(REQ_BODY, &models::ReviewVote {
                            user_id: "user id here".to_string(),
                            upvote: true,
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            },

            models::RouteList {
                file_name: "stats.md",
                routes: vec![
                    models::Route {
                        title: "Get Bot List Stats",
                        method: "GET",
                        path: "/stats",
                        description: r#"
Returns the bot list stats. This currently returns the full list of all bots
as a vector/list of IndexBot structs.

As a client, it is your responsibility, to parse this. Pagination may be added
if the list grows and then requires it."#,
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::ListStats {
                            bots: index_bots,
                            ..models::ListStats::default()
                        }),
                        auth_types: vec![],
                    }
                ]
            },

            models::RouteList {
                file_name: "commands.md",
                routes: vec![
                    models::Route {
                        title: "Add Command",
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

``target_type`` is a [TargetType](./enums#targettype)"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::TargetQuery {
                            target_type: models::TargetType::Bot,
                        }),
                        request_body: &body(REQ_BODY, &models::BotCommandVec {
                            commands: vec![models::BotCommand::default()],
                        }),
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::Bot],
                    },

                    models::Route {
                        title: "Delete Commands",
                        method: "DELETE",
                        path: "/bots/{id}/commands",
                        description: r#"
DELETE a command.

The ``id`` here must be the bot id you wish to add the command for

``names`` and ``ids`` must be a ``|`` seperated list of ``names`` or valid
UUIDs in the case of ids. Bad names/ids will be ignored"#,
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::CommandDeleteQuery {
                            nuke: Some(false),
                            names: Some("command name|command name 2".to_string()),
                            ids: Some("id 1|id 2".to_string()),
                        }),
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::Bot],
                    }
                ]
            }, 

            models::RouteList {
                file_name: "notifications.md",
                routes: vec![
                    models::Route {
                        title: "Get Notification Public Info",
                        method: "GET",
                        path: "/notifications/info",
                        description: "Get the public information required for creating a push notification",
                        path_params: &body(PATH_PARAMS, &models::FetchBotPath { id: 0 }),
                        query_params: &body(QUERY_PARAMS, &models::NotificationSub::default()),
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::NotificationInfo::default()),
                        auth_types: vec![],
                    },

                    models::Route {
                        title: "Create Push Notification Subscription",
                        method: "POST",
                        path: "/notifications/{id}/sub",
                        description: "Subscribes a user to a push notification.",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    },

                    models::Route {
                        title: "Create Test Push Notification",
                        method: "GET",
                        path: "/notifications/{id}/test",
                        description: "Creates a test push notification",
                        path_params: "",
                        query_params: "",
                        request_body: "",
                        response_body: &body(RESP_BODY, &models::APIResponse {
                            done: true,
                            reason: None,
                            context: None,
                        }),
                        auth_types: vec![models::RouteAuthType::User],
                    }
                ]
            }
        ]
    );
}







fn enum_doc<T: Debug + Serialize>(typ: T) -> String {
    format!("| **{:?}** | {} |\n", typ, json!(typ))
}

fn new_enum(data: models::EnumDesc) -> String {
    let mut keys = String::new();

    for name in data.alt_names {
        writeln!(&mut keys, "- ``{}``\n", name).unwrap();
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

// This is a doc file. Lots of lines are ok
#[allow(clippy::too_many_lines)]
pub fn document_enums() {
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

    // UserFlags
    docs += &new_enum(models::EnumDesc {
        name: "UserFlags",
        alt_names: vec!["flags"],
        description: "The flags of the user (such as vote privacy)",
        gen: || {
            let mut types = String::new();
            for typ in models::UserFlags::iter() {
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

    docs += "To see errors, please see https://github.com/Fates-List/api-v3/blob/main/src/models.rs and search for all ``APIError`` trait implementations";

    let path = match std::env::var_os("HOME") {
        None => {
            panic!("$HOME not set");
        }
        Some(path) => std::path::PathBuf::from(path),
    };

    let file_name = path.into_os_string().into_string().unwrap() + "/electrodragon/api-docs/endpoints/enums.md";

    let mut file = std::fs::File::create(file_name).unwrap();

    file.write_all(docs.as_bytes()).unwrap();
}
