use serde::{Deserialize, Serialize};
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

#[derive(Debug, Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy, Default)]
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

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct IndexBot {
    pub guild_count: i64,
    pub description: String,
    pub banner: Option<String>,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct Feature {
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
    pub features: HashMap<String, Feature>,
}

impl Index {
    pub fn new() -> Index {
        Index {
            top_voted: Vec::new(),
            certified: Vec::new(),
            new: Vec::new(),
            tags: Vec::new(),
            features: HashMap::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct IndexQuery {
    pub target_type: Option<String>,
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

#[derive(Deserialize, Serialize)]
pub struct Vanity {
    pub target_type: String,
    pub target_id: String
}

pub struct AppState {
    pub database: database::Database,
    pub docs: String,
}

#[derive(Deserialize, Serialize, Clone, Default)]
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