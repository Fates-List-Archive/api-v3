use serde::Serialize;
use std::collections::HashMap;
use crate::models;
use bevy_reflect::{Reflect, Struct};

fn _get_value_from(
    value: &dyn Reflect,
) -> String {    
    let mut field_name_ext: String = value.type_name().to_string();    

    // type_name replacer
    field_name_ext = field_name_ext.replace("core::option::Option", "Optional ");
    field_name_ext = field_name_ext.replace("alloc::string::", "");

    return "[".to_owned() + &field_name_ext + "]";
}

fn _get_params<T: Struct>(
    params: &T,
) -> String {
    let mut params_string = String::new();
    for (i, value) in params.iter_fields().enumerate() {
        let field_name: String = params.name_at(i).unwrap().to_string();
        let field_value = _get_value_from(value);
        params_string += &format!(
            "{field_name} {field_value}\n",
            field_name = field_name,
            field_value = field_value,
        )
    }
    return params_string;
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

    query_params_str += &("\n\n**Example**\n\n```json\n".to_string() + &String::from_utf8(ser4.into_inner()).unwrap() + "\n```");

    // Serialize path parameters
    let buf3 = Vec::new();
    let formatter3 = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser3 = serde_json::Serializer::with_formatter(buf3, formatter3);

    let mut path_params_str = _get_params(path_params);

    path_params.serialize(&mut ser3).unwrap();

    path_params_str += &("\n\n**Example**\n\n```json\n".to_string() + &String::from_utf8(ser3.into_inner()).unwrap() + "\n```");

    return format!(
        "# {title} ({method} {path})\n\n{description}\n\n**Query Parameters**\n\n{query_params}\n\n**Path Params**\n\n{path_params}\n\n**Request Body**\n\n```json\n{request_body}\n```\n\n**Response Body**\n\n```json\n{response_body}\n```\n\n\n",
        title = title,
        method = method,
        path = path,
        query_params = query_params_str,
        path_params = path_params_str,
        description = description,
        request_body = String::from_utf8(ser.into_inner()).unwrap(),
        response_body = String::from_utf8(ser2.into_inner()).unwrap(),
    );
}

pub fn document_routes() -> String {
    let mut docs: String = "# API v3\n".to_string();

    // TODO: For each route, add doc system

    // - Index route
    let mut index_bots = Vec::new();
    index_bots.push(models::IndexBot::default());
    let mut tags = Vec::new();
    tags.push(models::Tag::default());

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
            new: index_bots.clone(),
            tags: tags.clone(),
            features: HashMap::new(),
        },
    );

    // Return docs
    docs
}