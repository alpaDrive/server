extern crate types;

#[path = "./messages.rs"]
mod messages;

use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
use crate::sockets::Lobby;
use actix::{fut, ActorContext, ActorFutureExt};
use actix::{Actor, Addr, ContextFutureSpawner, Running, StreamHandler, WrapFuture};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::{CloseCode, Message::Text};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

// each ClientActorMessage will have a mode indicating the mode of that message
// Lobby uses this to send messages appropriately
#[derive(Clone)]
pub enum Mode {
    Broadcast,       // when the vehicle has to send a common message to all clients
    Whisper(String), // when the vehicle has to send a message to a specific client only without the others knowing
    Action,          // when the client has to order the vehicle to perform some specific action
    Request,         // when the client has to request certain specific data from the vehicle.
}

// each Connect message will have a sender indicating who that request came from
// used by lobby to handle connect requests appropriately
#[derive(Clone)]
pub enum Sender {
    Client(String),
    Admin,
    Pair(String),
}

// each WsMessage will have an action saying how to handle that message when it comes from Lobby
// ideally we could implement Handle<Disconnect> for WsConn but it's mostly unnecessary work that can be avoided
pub enum Action {
    Send,
    Disconnect(CloseCode),
    Pair,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMessage {
    mode: String,
    pub vid: String,
    pub conn_id: String,
    pub status: String,
    pub message: String,
    pub attachments: Vec<String>,
}

pub struct WsConn {
    id: String,
    lobby_addr: Addr<Lobby>,
    hb: Instant,
    room: String,
    sender: Sender,
}

fn draft_message(event: &str, message: &str, error: &str, conn_id: &str, uid: &str) -> String {
    json!({
        "event": event,
        "client": {
            "uid": uid,
            "conn_id": conn_id
         },
         "message": message,
         "error": error
   }).to_string()
}

impl ClientMessage {
    pub fn to_string(&self) -> String {
        json!({
            "mode": self.mode,
            "conn_id": self.conn_id,
            "vid": self.vid,
            "status": self.status,
            "message": self.message,
            "attachments": self.attachments
        }).to_string()
    }
    fn get_mode(&self) -> Result<Mode, String> {
        // just declaring these common fields to avoid repetition atm. Change later flexibly in the map
        let common = vec![self.status.clone(), self.conn_id.clone(), self.message.clone()];
        let map = HashMap::from([
            ("broadcast", (vec![self.status.clone(), self.message.clone()], Mode::Broadcast)),
            ("whisper", (common.clone(), Mode::Whisper(self.conn_id.clone()))),
            ("action", (common.clone(), Mode::Action)),
            ("request", (common, Mode::Request)),
        ]);

        match map.get(&*self.mode) {
            Some(values) => {
                let mut flag = true;
                for x in values.0.iter() {
                    if x.chars().count() < 1 {
                        flag = false;
                    }
                }
                if flag {
                    Ok(values.1.clone())
                } else {
                    Err(draft_message("error", "", "Your message is missing one or more parameters required for the given mode", &self.conn_id, ""))
                }
            },
            None => Err(draft_message("error", "", "Your message is missing or has an incorrect mode parameter", &self.conn_id, ""))
        }
    }
}

impl WsConn {
    pub fn new(room: String, id: String, lobby: Addr<Lobby>, sender: Sender) -> WsConn {
        WsConn {
            id,
            room,
            hb: Instant::now(),
            lobby_addr: lobby,
            sender,
        }
    }
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // println!("Disconnecting failed heartbeat");
                act.lobby_addr.do_send(Disconnect {
                    id: act.id.clone(),
                    room_id: act.room.clone(),
                    reason: None,
                });
                ctx.stop();
                return;
            }

            ctx.ping(b"PING");
        });
    }
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // do everything to place this actor in the lobby
        let addr = ctx.address();
        let recp = addr.recipient();
        self.lobby_addr
            .send(Connect {
                addr: recp,
                room_id: self.room.clone(),
                self_id: self.id.clone(),
                sender: self.sender.clone(),
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => (),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    // send disconnect message to the lobby and stop this actor
    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.lobby_addr.do_send(Disconnect {
            id: self.id.clone(),
            room_id: self.room.clone(),
            reason: None,
        });
        Running::Stop
    }
}

// message handler implementation
// pending work here to handle requests appropriately
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Ok(Text(s)) => {
                match serde_json::from_str::<ClientMessage>(&s) {
                    Ok(value) => {
                        match value.get_mode() {
                            Ok(mode) => self.lobby_addr.do_send(ClientActorMessage {
                                id: self.id.clone(),
                                msg: value,
                                room_id: self.room.clone(),
                                mode
                            }),
                            Err(error) => ctx.text(error)
                        }
                    }
                    Err(_) => ctx.text(draft_message("error", "", "This message is not in the specified format", &self.id, "")),
                };
            }
            Err(_) => {}
        }
    }
}

// this is how messages are sent to the client
// the server puts a message in the actor's mailbox
impl Handler<WsMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        match msg.action {
            Action::Send => ctx.text(msg.message),
            Action::Disconnect(code) => {
                ctx.close(Some(ws::CloseReason {
                    code,
                    description: Some(msg.message),
                }));
                ctx.stop();
            }
            Action::Pair => {}
        };
    }
}
