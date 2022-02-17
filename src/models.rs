use serde::{Deserialize, Serialize};
use num_enum::TryFromPrimitive;
use serde_repr::*;
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub disc: String,
    pub avatar: String,
    pub bot: bool,
}

#[derive(Debug, Eq, TryFromPrimitive, Serialize_repr, Deserialize_repr, PartialEq, Clone, Copy)]
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

#[derive(Deserialize, Serialize)]
pub struct IndexBot {
    pub guild_count: i64,
    pub description: String,
    pub banner: Option<String>,
    pub nsfw: bool,
    pub votes: i64,
    pub state: State,
    pub user: User,
}

#[derive(Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    pub iconify_data: String,
    pub id: String,
    pub owner_guild: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Feature {
    pub name: String,
    pub viewed_as: String,
    pub description: String,
}

#[derive(Deserialize, Serialize)]
pub struct Index {
    pub top_voted: Vec<IndexBot>,
    pub certified: Vec<IndexBot>,
    pub tags: Vec<Tag>,
    pub features: HashMap<String, Feature>,
}

#[derive(Deserialize, Serialize)]
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