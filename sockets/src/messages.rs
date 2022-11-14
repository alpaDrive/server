use actix::prelude::{Message, Recipient};// use serde_json::Value;
use crate::ws::{Sender, Action, Mode, ClientMessage};

//WsConn responds to this to pipe it through to the actual client
#[derive(Message)]
#[rtype(result = "()")]

pub struct WsMessage {
    pub message: String,
    pub id: String,
    pub action: Action
}

//WsConn sends this to the lobby to say "put me in please"
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub room_id: String,
    pub self_id: String,
    pub sender: Sender
}

//WsConn sends this to a lobby to say "take me out please"
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub room_id: String,
    pub id: String,
    pub reason: Option<String>
}

//client sends this to the lobby for the lobby to echo out.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub id: String,
    pub msg: ClientMessage,
    pub room_id: String,
    pub mode: Mode
}