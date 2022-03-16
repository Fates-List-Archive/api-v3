
   
use std::time::{Duration, Instant};
use actix_web::{HttpRequest, get, web, HttpResponse, Error};

use actix_ws::Message;
use crate::models;
use crate::converters;
use futures::StreamExt;
use log::{debug, error};
use redis::AsyncCommands;
use serde::Deserialize;
use std::collections::HashMap;
use sqlx::postgres::PgPool;

#[get("/ws/_preview")]
pub async fn preview(req: HttpRequest, body: web::Payload) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    
    let mut close_reason = None;

    actix_rt::spawn(async move {
        let mut hb = Instant::now();

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Message::Pong(_) => {
                    hb = Instant::now();
                }        
                Message::Text(text) => {
                    if text == "PING" && session.text(Instant::now().duration_since(hb).as_micros().to_string()).await.is_err() {
                        break;
                    }

                    let data: models::PreviewRequest = serde_json::from_str(&text).unwrap_or_else(|_| models::PreviewRequest {
                        long_description_type: models::LongDescriptionType::Html,
                        text: "".to_string()
                    });

                    if data.text.is_empty() {
                        continue;
                    }

                    if session.text(serde_json::to_string(&models::PreviewResponse {
                        preview: converters::sanitize_description(data.long_description_type, data.text)
                    }).unwrap()).await.is_err() {
                        break;
                    }
                }

                Message::Close(reason) => {
                    close_reason = reason;
                    break;
                } 

                _ => break      
            }
        }

        let _ = session.close(close_reason).await;

    });

    Ok(response)
}

#[derive(Deserialize, Copy, Clone)]
pub enum WsMode {
    Bot,
    Server,
}

#[derive(Deserialize)]
pub struct WsModeStruct {
    mode: WsMode,
}

async fn bot_gateway_task_sub(mode: WsMode, id: i64, session: actix_ws::Session) {
    let client = redis::Client::open("redis://127.0.0.1:1001/1").unwrap();

    let mut pubsub_conn = client.get_async_connection().await.unwrap().into_pubsub();

    let mode = match mode {
        WsMode::Bot => "bot",
        WsMode::Server => "server",
    };

    let res = pubsub_conn.subscribe(mode.to_string()+"-"+&id.to_string()).await;

    if res.is_err() {
        error!("{}", res.err().unwrap());
        return;
    }

    let mut session = session.clone();

    session.text("GWTASK LISTEN").await.unwrap();


    while let Some(msg) = pubsub_conn.on_message().next().await {
        let msg: Result<String, _> = msg.get_payload();
        if msg.is_err() {
            continue;
        }
        if session.text(msg.unwrap()).await.is_err() {
            return;
        }
    }


}

async fn bot_gateway_task_archive(pool: PgPool, mode: WsMode, id: i64, session: actix_ws::Session) {
    let mode = match mode {
        WsMode::Bot => "bot",
        WsMode::Server => "server",
    };

    let mut session = session.clone();

    session.text("GWTASK ARCHIVE").await.unwrap();

    // Now we get every event from redis
    let rows = sqlx::query!(
        "SELECT event FROM events WHERE id = $1 AND type = $2",
        id,
        mode
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for row in rows {
        let event = serde_json::to_string(&row.event);
        if event.is_err() {
            error!("{} {}", id.to_string(), event.err().unwrap());
            continue;
        }
        if session.text(event.unwrap()).await.is_err() {
            return;
        }
    }

    if session.text("GWTASKACK ARCHIVE").await.is_err() {
        error!("{}", "GWTASKACK ARCHIVE could not be sent");
    }
}

#[get("/ws/{id}")]
pub async fn bot_ws(req: HttpRequest, id: web::Path<i64>, mode: web::Query<WsModeStruct>, body: web::Payload) -> Result<HttpResponse, Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    
    let data: &models::AppState = req.app_data::<web::Data<models::AppState>>().unwrap();

    let pool = data.database.get_postgres();

    let mut close_reason = None;
    let mut gw_task = None;

    actix_rt::spawn(async move {
        let id = id.into_inner();
        let mode = mode.into_inner().mode;
        let mut hb = Instant::now();

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Message::Pong(_) => {
                    hb = Instant::now();
                }        
                Message::Text(text) => {
                    if text == "PING" && session.text(Instant::now().duration_since(hb).as_micros().to_string()).await.is_err() {
                        break;
                    }

                    if text == "SUB" {
                        if gw_task.is_some() {
                            // Error out, you can only have one gateway task per session
                            close_reason = Some(actix_ws::CloseReason {
                                code: actix_ws::CloseCode::Other(4001),
                                description: Some("You can only have one gateway task per session at any given time".to_string())
                            });
                            break;
                        }
                        // Subscribe to messages sent to the bots websocket channel
                        gw_task = Some(actix_rt::spawn(bot_gateway_task_sub(mode, id, session.clone())));
                    } else if text == "ARCHIVE" {
                        if gw_task.is_some() {
                            // Error out, you can only have one gateway task per session
                            close_reason = Some(actix_ws::CloseReason {
                                code: actix_ws::CloseCode::Other(4001),
                                description: Some("You can only have one gateway task per session at any given time".to_string())
                            });
                            break;
                        }
                        // Subscribe to messages sent to the bots websocket channel
                        gw_task = Some(actix_rt::spawn(bot_gateway_task_archive(pool.clone(), mode, id, session.clone())));
                    } else if text == "ENDGWTASK" {
                        if gw_task.is_none() {
                            // Error out, cannot UNSUB if you are not subscribed
                            close_reason = Some(actix_ws::CloseReason {
                                code: actix_ws::CloseCode::Other(4002),
                                description: Some("You can only unsubscribe if you actually have a gateway task running".to_string())
                            });
                            break;
                        }
                        if gw_task.is_some() {
                            gw_task.unwrap().abort();
                        }                        
                        gw_task = None;
                        if session.text("GWTASK NONE").await.is_err() {
                            break;
                        }        
                    }
                }

                Message::Close(reason) => {
                    close_reason = reason;
                    break;
                } 

                _ => break      
            }
        }

        let _ = session.close(close_reason).await;
        if gw_task.is_some() {
            gw_task.unwrap().abort();
        }    

    });

    Ok(response)
}