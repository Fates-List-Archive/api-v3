extern crate redis;
use uuid::Uuid;
use redis::AsyncCommands;
use redis::Commands;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models;

struct IpcCall {
    redis: redis::Client,
    cmd: String,
    args: Vec<String>,
    message: String, // Use serde_json::to_string(&message) to serialize,
    timeout: u64, // Use 0 for no timeout
}

enum IpcErr {
    Timeout,
    Ok,
    Unknown,
}

async fn ipc_call(call: &mut IpcCall) -> Result<String, IpcErr> {
    let cmd_id: String = Uuid::new_v4().to_hyphenated().to_string();
    let mut conn = call.redis.get_async_connection().await.unwrap();
    if !call.message.is_empty() {
        let msg_id: String = Uuid::new_v4().to_hyphenated().to_string();
        let message: String = cmd_id.clone() + " " + &call.cmd + " " + &msg_id;
        let _: () = conn.set(msg_id.clone(), &call.message).await.unwrap();
        call.args.push(msg_id.clone());
    } 

    let message: String = cmd_id.clone() + " " + &call.cmd + " " + &call.args.join(" ");
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

async fn ipc_call_sync(call: &mut IpcCall) -> Result<String, IpcErr> {
    let cmd_id: String = Uuid::new_v4().to_hyphenated().to_string();
    let mut conn = call.redis.get_connection().unwrap();
    if !call.message.is_empty() {
        let msg_id: String = Uuid::new_v4().to_hyphenated().to_string();
        let message: String = cmd_id.clone() + " " + &call.cmd + " " + &msg_id;
        let _: () = conn.set(msg_id.clone(), &call.message).unwrap();
        call.args.push(msg_id.clone());
    } 

    let message: String = cmd_id.clone() + " " + &call.cmd + " " + &call.args.join(" ");
    let _: () = conn.publish("_worker_fates".to_string(), message).unwrap();
    if call.timeout > 0 {
        let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        while SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - start < call.timeout {
            let data: String = conn.get(cmd_id.clone()).unwrap_or("".to_string());
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

pub async fn get_user(redis: redis::Client, user_id: i64) -> models::User {
    let mut call = IpcCall {
        redis: redis,
        cmd: "GETCH".to_string(),
        args: vec![user_id.to_string()],
        message: "".to_string(),
        timeout: 0,
    };
    let val = ipc_call(&mut call).await;
}