use serde::{Deserialize, Serialize};
use num_enum::TryFromPrimitive;
use serde_repr::*;
use std::collections::HashMap;
use thiserror::Error;
use crate::database;
use actix_web::{http, HttpResponse, error::ResponseError};
use paperclip::actix::{
    // extension trait for actix_web::App and proc-macro attributes
    OpenApiExt, Apiv2Schema, api_v2_operation,
    // If you prefer the macro syntax for defining routes, import the paperclip macros
    // get, post, put, delete
    // use this instead of actix_web::web
    web::{self, Json},
};

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct User {
    pub id: String,
    pub username: String,
    pub disc: String,
    pub avatar: String,
    pub bot: bool,
}

#[derive(Debug, Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Apiv2Schema)]
#[repr(i32)]
pub enum State {
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

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct IndexBot {
    pub guild_count: i64,
    pub description: String,
    pub banner: Option<String>,
    pub nsfw: bool,
    pub votes: i64,
    pub state: State,
    pub user: User,
}

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct Tag {
    pub name: String,
    pub iconify_data: String,
    pub id: String,
    pub owner_guild: Option<String>,
}

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct Feature {
    pub name: String,
    pub viewed_as: String,
    pub description: String,
}

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct Index {
    pub new: Vec<IndexBot>,
    pub top_voted: Vec<IndexBot>,
    pub certified: Vec<IndexBot>,
    pub tags: Vec<Tag>,
    pub features: HashMap<String, Feature>,
}

#[derive(Deserialize, Serialize, Apiv2Schema)]
pub struct IndexQuery {
    pub target_type: Option<String>,
}

#[derive(Deserialize, Serialize, PartialEq)]
pub enum Status {
    Unknown = 0,
    Online = 1,
    Offline = 2, // Or invisible
    Idle = 3,
    DoNotDisturb = 4,
}

#[derive(Deserialize, Serialize)]
pub struct Vanity {
    pub target_type: String,
    pub target_id: String
}

pub struct AppState {
    pub database: database::Database,
}

#[derive(Deserialize, Serialize)]
struct APIResponse {
    done: bool,
    reason: Option<String>,
    error: Option<String>, // This is the error itself
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
            error: Some(self.name()),
            done: status_code.is_success(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}  