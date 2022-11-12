extern crate types;

#[path ="./messages.rs"]
mod messages;

use actix::{fut, ActorContext, ActorFutureExt};
use actix::{Actor, Addr, Running, StreamHandler, WrapFuture, ContextFutureSpawner};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use actix_web_actors::ws::Message::Text;
use std::time::{Duration, Instant};
use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage, Action};
use crate::sockets::Lobby;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub enum Mode {
    Client,
    Admin,
    Pair(String)
}

pub struct WsConn {
    id: String,
    lobby_addr: Addr<Lobby>,
    hb: Instant,
    room: String,
    mode: Mode
}

impl WsConn {
    pub fn new(room: String, id: String, lobby: Addr<Lobby>, mode: Mode) -> WsConn {
        WsConn {
            id: id,
            room,
            hb: Instant::now(),
            lobby_addr: lobby,
            mode
        }
    }
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // println!("Disconnecting failed heartbeat");
                act.lobby_addr.do_send(Disconnect { id: act.id.clone(), room_id: act.room.clone(), reason: None });
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
                mode: self.mode.clone(),
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
        self.lobby_addr.do_send(Disconnect { id: self.id.clone(), room_id: self.room.clone(), reason: None });
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
            Ok(Text(s)) => self.lobby_addr.do_send(ClientActorMessage {
                id: self.id.clone(),
                msg: s.to_string(),
                room_id: self.room.clone()
            }),
            Err(_) => {},
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
                    code: code,
                    description: Some(msg.message)
                }));
                ctx.stop();
            },
            Action::Pair => {}
        };
    }
}
