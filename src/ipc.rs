use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models;
use deadpool_redis::redis::AsyncCommands;
use serde::Serialize;
use log::{error, debug};

struct IpcCall {
    redis: deadpool_redis::Pool,
    cmd: String,
    args: Vec<String>,
    message: String, // Use serde_json::to_string(&message) to serialize,
    timeout: u64, // Use 0 for no timeout
}

#[derive(Debug)]
enum IpcErr {
    Timeout,
}

async fn ipc_call(call: &mut IpcCall) -> Result<String, IpcErr> {
    let cmd_id: String = Uuid::new_v4().to_hyphenated().to_string();
    let mut conn = call.redis.get().await.unwrap();
    if !call.message.is_empty() {
        let msg_id: String = Uuid::new_v4().to_hyphenated().to_string();
        let _: () = conn.set(msg_id.clone(), &call.message).await.unwrap();
        call.args.push(msg_id.clone());
    } 

    let message: String = call.cmd.clone() + " " + &cmd_id.clone() + " " + &call.args.join(" ");
    let _: () = conn.publish("_worker_fates".to_string(), message).await.unwrap();
    if call.timeout > 0 {
        let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        while SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - start < call.timeout {
            let data: String = conn.get(cmd_id.clone()).await.unwrap_or_else(|_| "".to_string());
            if data.is_empty() {
                continue
            } else {
                return Ok(data);
            }
        }
        Err(IpcErr::Timeout)
    } else {
        Err(IpcErr::Timeout)
    }
}

/// Use 0 if user_id is unset
pub async fn resolve_guild_invite(redis: deadpool_redis::Pool, guild_id: i64, user_id: i64) -> String {
    let mut call = IpcCall {
        redis,
        cmd: "GUILDINVITE".to_string(),
        args: vec![guild_id.to_string(), user_id.to_string()],
        message: "".to_string(),
        timeout: 30,
    };
    let res = ipc_call(&mut call).await;
    match res {
        Ok(res) => {
            debug!("GuildInviteResolver Response: {:?}", res);
            res
        }
        Err(err) => {
            debug!("GuildInviteResolver Response: {:?}", err);
            format!("{:?}", err)
        }
    }
}

/// Gets a user
pub async fn get_user(redis: deadpool_redis::Pool, user_id: i64) -> models::User {
    // First check cache
    let mut conn = redis.get().await.unwrap();
    let data: String = conn.get("user-cache:".to_string() + &user_id.to_string()).await.unwrap_or_else(|_| "".to_string());
    if !data.is_empty() {
        let user: Option<models::User> = serde_json::from_str(&data).unwrap_or(None);
        if user.is_some() {
            return user.unwrap();
        }
    }

    // Then call baypaw (http://localhost:1234/getch/928702343732658256)
    let req = reqwest::Client::builder()
    .user_agent("DiscordBot (https://fateslist.xyz, 0.1) FatesList-Lightleap-WarriorCats")
    .build()
    .unwrap()
    .get("http://localhost:1234/getch/".to_string() + &user_id.to_string())
    .timeout(std::time::Duration::from_secs(30));

    let res = req.send().await.unwrap();

    let user: models::User = res.json().await.unwrap_or_else(|_| models::User {
        id: "".to_string(),
        username: "Unknown User".to_string(),
        disc: "0000".to_string(),
        avatar: "https://api.fateslist.xyz/static/botlisticon.webp".to_string(),
        bot: false,
    });
    
    if user.id.is_empty() {
        conn.set_ex("user-cache:".to_string() + &user_id.to_string(), serde_json::to_string(&user).unwrap(), 60*60*1).await.unwrap_or_else(|_| "".to_string());
    } else {
        conn.set_ex("user-cache:".to_string() + &user_id.to_string(), serde_json::to_string(&user).unwrap(), 60*60*8).await.unwrap_or_else(|_| "".to_string());
    }
    return user;
}
