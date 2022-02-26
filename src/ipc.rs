use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};
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