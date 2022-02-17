use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models;
use serde_json;
use deadpool_redis;
use deadpool_redis::redis::AsyncCommands;

struct IpcCall {
    redis: deadpool_redis::Pool,
    cmd: String,
    args: Vec<String>,
    message: String, // Use serde_json::to_string(&message) to serialize,
    timeout: u64, // Use 0 for no timeout
}

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
            let data: String = conn.get(cmd_id.clone()).await.unwrap_or("".to_string());
            if data.is_empty() {
                continue
            } else {
                return Ok(data);
            }
        }
        return Err(IpcErr::Timeout);
    } else {
        Err(IpcErr::Timeout)
    }
}

pub async fn get_user(redis: deadpool_redis::Pool, user_id: i64) -> models::User {
    // First check cache
    let mut conn = redis.get().await.unwrap();
    let data: String = conn.get("user-cache:".to_string() + &user_id.to_string()).await.unwrap_or("".to_string());
    if !data.is_empty() {
        let user: models::User = serde_json::from_str(&data).unwrap();
        return user;
    }
    
    let mut call = IpcCall {
        redis: redis,
        cmd: "GETCH".to_string(),
        args: vec![user_id.to_string()],
        message: "".to_string(),
        timeout: 30,
    };
    let val = ipc_call(&mut call).await;
    match val {
        Ok(data) => {
            conn.set_ex("user-cache:".to_string() + &user_id.to_string(), data.clone(), 60*60*8).await.unwrap_or("".to_string());
            let user: models::User = serde_json::from_str(&data).unwrap();
            user
        },
        Err(_) => {
            models::User {
                id: "0".to_string(),
                username: "Unknown User".to_string(),
                disc: "0000".to_string(),
                avatar: "https://api.fateslist.xyz/static/botlisticon.webp".to_string(),
                bot: false,
            }
        }
    }
}
