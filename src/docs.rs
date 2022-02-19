use serde::Serialize;
use crate::models;
use bevy_reflect::{Reflect, Struct};

fn _get_value_from(
    value: &dyn Reflect,
) -> String {
    let mut field_name_ext: String = value.type_name().to_string();    

    // type_name replacer
    field_name_ext = field_name_ext.replace("core::option::Option", "Optional ");
    field_name_ext = field_name_ext.replace("alloc::string::", "");

    // Optional string case
    if let Some(value) = value.downcast_ref::<Option<String>>() {
        match value {
            Some(value) => {
                field_name_ext = "String? ".to_string() + "| default = " + value;
            },
            None => {
                // Ignored
            },
        }
    }

    // Optional i64 case
    if let Some(value) = value.downcast_ref::<Option<i64>>() {
        match value {
            Some(value) => {
                field_name_ext = "i64? ".to_string() + "| default = " + &value.to_string();
            },
            None => {
                // Ignored
            },
        }
    }    

    "[".to_owned() + &field_name_ext + " (type info may be incomplete, see example)]"
}

fn _get_params<T: Struct>(
    params: &T,
) -> String {
    let mut params_string = String::new();
    for (i, value) in params.iter_fields().enumerate() {
        let field_name: String = params.name_at(i).unwrap().to_string();
        let field_value = _get_value_from(value);
        params_string += &format!(
            "- **{field_name}** {field_value}\n",
            field_name = field_name,
            field_value = field_value,
        )
    }
    params_string
}

fn doc<T: Serialize, T2: Serialize, T3: Struct + Serialize, T4: Struct + Serialize>(
    title: &str,
    method: &str,
    path: &str,
    path_params: &T3,
    query_params: &T4,
    description: &str,
    request_body: &T,
    response_body: &T2,
    equiv_v2_route: &str,
) -> String {
    // Serialize request body
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
    
    request_body.serialize(&mut ser).unwrap();

    // Serialize response body
    let buf2 = Vec::new();
    let formatter2 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser2 = serde_json::Serializer::with_formatter(buf2, formatter2);

    response_body.serialize(&mut ser2).unwrap();

    // Serialize query parameters
    let buf4 = Vec::new();
    let formatter4 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser4 = serde_json::Serializer::with_formatter(buf4, formatter4);

    let mut query_params_str = _get_params(query_params);

    query_params.serialize(&mut ser4).unwrap();

    let query_params_json = &String::from_utf8(ser4.into_inner()).unwrap();

    query_params_str += &("\n\n**Example**\n\n```json\n".to_string() + &query_params_json.clone() + "\n```");

    // Serialize path parameters
    let buf3 = Vec::new();
    let formatter3 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser3 = serde_json::Serializer::with_formatter(buf3, formatter3);

    let mut path_params_str = _get_params(path_params);

    path_params.serialize(&mut ser3).unwrap();

    let path_params_json = &String::from_utf8(ser3.into_inner()).unwrap();

    path_params_str += &("\n\n**Example**\n\n```json\n".to_string() + &path_params_json.clone() + "\n```");

    let mut base_doc = format!(
        "## {title}\n### {method} {path}\n\n{description}\n\n**API v2 analogue:** {equiv_v2_route}",
        title = title,
        method = method,
        path = path,
        description = description,
        equiv_v2_route = equiv_v2_route,
    );

    if path_params_json.len() > 2 {
        base_doc += &("\n\n**Path parameters**\n\n".to_string() + &path_params_str);
    }
    if query_params_json.len() > 2 {
        base_doc += &("\n\n**Query parameters**\n\n".to_string() + &query_params_str);
    }

    return base_doc + &format!(
        "\n\n**Request Body**\n\n```json\n{request_body}\n```\n\n**Response Body**\n\n```json\n{response_body}\n```\n\n\n",
        request_body = String::from_utf8(ser.into_inner()).unwrap(),
        response_body = String::from_utf8(ser2.into_inner()).unwrap(),
    );
}

pub fn document_routes() -> String {
    let mut docs: String = "# API v3\n**API URL**: ``https://next.fateslist.xyz`` (for now)\n".to_string();

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
    }.serialize(&mut ser).unwrap();

    docs += &("\n## Base Response\n\nA default API Response will be of the below format:\n\n```json\n".to_string() + &String::from_utf8(ser.into_inner()).unwrap() + "\n```\n\n");

    // TODO: For each route, add doc system


    // - Index route
    let index_bots = vec![models::IndexBot::default()];

    let tags = vec![models::Tag::default()];

    let features = vec![models::Feature::default()];

    docs += &doc(
        "Index",
        "GET",
        "/index",
        &models::Empty {},
        &models::IndexQuery {
            target_type: Some("bot".to_string()),
        },
        "Returns the index for bots and servers",
        &models::Empty {},
        &models::Index {
            top_voted: index_bots.clone(),
            certified: index_bots.clone(),
            new: index_bots, // Clone later if needed
            tags,
            features,
        },
        "(no longer working) [Get Index](https://api.fateslist.xyz/docs/redoc#operation/get_index)",
    );


    // - Vanity route
    docs += &doc(
        "Resolve Vanity",
        "GET",
        "/code/{code}",
        &models::VanityPath {
            code: "my-vanity".to_string(),
        },
        &models::Empty {},
        "Resolves the vanity for a bot/server in the list",
        &models::Empty {},
        &models::Vanity {
            target_id: "0000000000".to_string(),
            target_type: "bot | server".to_string(),
        },
        "(no longer working) [Get Vanity](https://api.fateslist.xyz/docs/redoc#operation/get_vanity)",
    );

    // - Fetch Bot route
    docs += &doc(
        "Get Bot",
        "GET",
        "/bots/{id}",
        &models::FetchBotPath::default(),
        &models::FetchBotQuery::default(),
r#"
Fetches bot information given a bot ID. If not found, 404 will be returned. 

This endpoint handles both bot IDs and client IDs

Differences from API v2:

- Unlike API v2, this does not support compact or no_cache. Owner order is also guaranteed
- *``long_description/css`` is sanitized with ammonia by default, use `long_description_raw` if you want the unsanitized version*
- All responses are cached for a short period of time. There is *no* way to opt out unlike API v2
- Some fields have been renamed or removed (such as ``promos`` which may be readded at a later date)

**Set the Frostpaw header if you are a custom client**
"#,
    &models::Empty{},
    &models::Bot::default(), // TODO
    "[Fetch Bot](https://api.fateslist.xyz/docs/redoc#operation/fetch_bot)",
    );

    // - Search List route
    docs += &doc(
        "Search List",
        "GET",
        "/search?q={query}",
        &models::Empty {},
        &models::SearchQuery {
            q: Some("mew".to_string()),
        },
        r#"Searches the list"#,
        &models::Empty {},
        &models::Search {
            bots: vec![models::IndexBot::default()],
            servers: vec![models::IndexBot::default()],
            packs: vec![models::BotPack::default()],
            profiles: vec![models::SearchProfile::default()],
            tags: models::SearchTags {
                bots: vec![models::Tag::default()],
                servers: vec![models::Tag::default()]
            },
        },
        "(no longer working) [Fetch Bot](https://api.fateslist.xyz/docs/redoc#operation/search_list)",
    );

    // Return docs
    docs
}