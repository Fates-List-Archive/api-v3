use serde;
use std::collections::HashMap;
use crate::models;
fn doc<T: serde::Serialize, T2: serde::Serialize>(
    title: &str,
    path: &str,
    description: &str,
    request_body: &T,
    response_body: &T2,
) -> String {
    let buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(buf, formatter);
    
    request_body.serialize(&mut ser).unwrap();

    let buf2 = Vec::new();
    let formatter2 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser2 = serde_json::Serializer::with_formatter(buf2, formatter2);

    response_body.serialize(&mut ser2).unwrap();

    return format!(
        "# {title} ({path})\n\n{description}\n\n**Request Body**\n```json\n{request_body}\n```\n\n**Response Body**\n```json\n{response_body}\n```",
        title = title,
        path = path,
        description = description,
        request_body = String::from_utf8(ser.into_inner()).unwrap(),
        response_body = String::from_utf8(ser2.into_inner()).unwrap(),
    );
}

pub fn document_routes() -> String {
    let mut docs: String = "".to_string();

    // TODO: For each route, add doc system

    // - Index route
    let mut index_bots = Vec::new();
    index_bots.push(models::IndexBot::default());
    let mut tags = Vec::new();
    tags.push(models::Tag::default());

    docs += &doc(
        "Index",
        "/index",
        "Returns the index for bots and servers",
        &models::IndexQuery {
            target_type: Some("bot".to_string()),
        },
        &models::Index {
            top_voted: index_bots.clone(),
            certified: index_bots.clone(),
            new: index_bots.clone(),
            tags: tags.clone(),
            features: HashMap::new(),
        },
    );

    // Return docs
    docs
}