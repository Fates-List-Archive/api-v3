use std::time::{Duration, Instant};
use actix_web::{HttpRequest, get, web, HttpResponse, Error};

use actix::prelude::*;
use actix_web_actors::ws;
use crate::models;
use crate::converters;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(PartialEq)]
enum WsMode {
    Preview,
}

use log::error;

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct FatesWebsocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    mode: WsMode,
}

impl FatesWebsocket {
    fn new(mode: WsMode) -> Self {
        Self { hb: Instant::now(), mode }
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                error!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for FatesWebsocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for FatesWebsocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // process websocket messages
        println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                if self.mode == WsMode::Preview {
                    let data: models::PreviewRequest = serde_json::from_str(&text).unwrap_or_else(|_| models::PreviewRequest {
                        long_description_type: models::LongDescriptionType::Html,
                        text: "".to_string()
                    });

                    ctx.text(serde_json::to_string(&models::PreviewResponse {
                        preview: converters::sanitize_description(data.long_description_type, data.text)
                    }).unwrap())           
                }
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

#[get("/ws/_preview")]
pub async fn preview(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(FatesWebsocket::new(WsMode::Preview), &req, stream)
}