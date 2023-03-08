pub mod messages;
pub mod ws;

pub mod sockets {
    use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
    use crate::ws::{Action, Mode, Sender};
    use actix::prelude::{Actor, Handler, Recipient};
    use actix::{Addr, SyncArbiter, SyncContext};
    use actix_web_actors::ws::CloseCode;
    use serde_json::json;
    use std::collections::hash_map::Entry;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, RwLock};

    pub struct Lobby {
        sessions: HashMap<String, Recipient<WsMessage>>,
        rooms: HashMap<String, HashSet<String>>, //room id  to list of users id
        admins: HashMap<String, String>,
        lock: Arc<RwLock<HashMap<String, String>>>,
        sessions_lock: Arc<RwLock<usize>>,
    }

    impl Default for Lobby {
        fn default() -> Self {
            Lobby {
                sessions: HashMap::new(),
                rooms: HashMap::new(),
                admins: HashMap::new(),
                lock: Arc::new(RwLock::new(HashMap::new())),
                sessions_lock: Arc::new(RwLock::new(0)),
            }
        }
    }

    impl Clone for Lobby {
        fn clone(&self) -> Self {
            Lobby {
                sessions: self.sessions.clone(),
                rooms: self.rooms.clone(),
                admins: self.admins.clone(),
                lock: self.lock.clone(),
                sessions_lock: self.sessions_lock.clone(),
            }
        }
    }

    impl Lobby {
        pub fn new(
            lock: Arc<RwLock<HashMap<String, String>>>,
            sessions: Arc<RwLock<usize>>,
        ) -> Addr<Self> {
            let lobby = Lobby {
                sessions: HashMap::new(),
                rooms: HashMap::new(),
                admins: HashMap::new(),
                lock: lock.clone(),
                sessions_lock: sessions,
            };
            let addr = SyncArbiter::start(1, move || lobby.clone());
            addr
        }

        // send message to user in the room
        fn send_message(&self, message: &str, id_to: &String) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient.do_send(WsMessage {
                    message: message.to_string(),
                    id: id_to.to_string(),
                    action: Action::Send,
                });
            }
        }
        // send disconnect to an existing user in the room
        fn send_disconnect(&self, reason: &str, id_to: &String, code: CloseCode) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient.do_send(WsMessage {
                    message: reason.to_string(),
                    id: id_to.to_string(),
                    action: Action::Disconnect(code),
                });
            }
        }
        // disconnect user before entering the room (special cases liek pair)
        fn send_disconnect_standalone(
            &self,
            reason: String,
            recipient: &Recipient<WsMessage>,
            id_to: String,
            code: CloseCode,
        ) {
            let _ = recipient.do_send(WsMessage {
                message: reason,
                id: id_to,
                action: Action::Disconnect(code),
            });
        }
        // send a message to the admin vehicle
        fn message_vehicle(&mut self, room: String, message: String) {
            let sessions = self.sessions.clone();
            if let Entry::Occupied(admin) = self.admins.entry(room) {
                if let Some(socket_recipient) = sessions.get(admin.get()) {
                    let _ = socket_recipient.do_send(WsMessage {
                        message: message,
                        id: admin.get().to_string(),
                        action: Action::Send,
                    });
                }
            }
        }
        // add an actor to the sessions map
        fn insert(&mut self, self_id: String, addr: Recipient<WsMessage>, uid: &str) {
            self.sessions.insert(self_id.clone(), addr.clone());
            self.send_message(
                &json!({
                     "event": "connect",
                     "client": {
                         "uid": uid,
                         "conn_id": self_id
                      },
                      "message": "Connection successful",
                      "error": ""
                })
                .to_string(),
                &self_id,
            );
        }
        // when called from a WsConn actor, sends the message to everyone else
        fn broadcast(&self, message: String, room: String, id: String) {
            self.rooms
                .get(&room)
                .unwrap()
                .iter()
                .filter(|conn_id| *conn_id.to_owned() != id)
                .for_each(|user_id| self.send_message(&*message, user_id));
        }
        // given and room and a target id inside it, sends the message to that id only
        fn whisper(&self, message: String, room: String, target: String) {
            self.rooms
                .get(&room)
                .unwrap()
                .iter()
                .filter(|conn_id| *conn_id.to_owned() == target)
                .for_each(|user_id| self.send_message(&*message, user_id));
        }
    }

    impl Actor for Lobby {
        type Context = SyncContext<Self>;
    }

    // Handler for Disconnect message.
    impl Handler<Disconnect> for Lobby {
        type Result = ();

        fn handle(&mut self, msg: Disconnect, _: &mut SyncContext<Self>) {
            if self.sessions.remove(&msg.id).is_some() {
                if let Entry::Occupied(admin) = self.admins.entry(msg.room_id.clone()) {
                    if &msg.id == admin.get() {
                        let rooms = self.rooms.get(&msg.room_id).unwrap();
                        let mut sessions_guard = self.sessions_lock.write().unwrap();
                        let sessions = *sessions_guard;
                        *sessions_guard = sessions - rooms.len();
                        rooms.iter()
                        .filter(|conn_id| *conn_id.to_owned() != msg.id)
                        .for_each(|user_id| self.send_disconnect(&json!({"event": "disconnect", "client": { "uid": "", "conn_id": user_id }, "message": "Vehicle left and the room is being closed", "error": ""}).to_string(), user_id, CloseCode::Normal));
                        let mut admins = self.lock.write().unwrap();
                        admins.remove_entry(&msg.room_id);
                        drop(admins);
                        self.rooms.remove(&msg.room_id);
                        self.admins.remove(&msg.room_id);
                    } else {
                        self.message_vehicle(msg.room_id.clone(), json!({"event": "disconnect", "client": { "uid": "", "conn_id": msg.id }, "message": "A client has disconnected", "error": "" }).to_string());
                        if let Some(lobby) = self.rooms.get_mut(&msg.room_id) {
                            lobby.remove(&msg.id);
                        }
                        let mut sessions_guard = self.sessions_lock.write().unwrap();
                        let sessions = *sessions_guard;
                        *sessions_guard = sessions - 1;
                    }
                }
            }
        }
    }

    // handler for connect message
    // pending work here to manage room creation/joining appropriately
    impl Handler<Connect> for Lobby {
        type Result = ();

        fn handle(&mut self, msg: Connect, _: &mut SyncContext<Self>) -> Self::Result {
            // if it's a vehicle issuing connect
            match self.rooms.entry(msg.room_id.clone()) {
                Entry::Occupied(mut o) => {
                    match msg.sender {
                        Sender::Client(uid) => {
                            o.get_mut().insert(msg.self_id.clone());
                            self.insert(msg.self_id.clone(), msg.addr, &msg.room_id);
                            self.message_vehicle(msg.room_id, json!({"event": "connected", "client": {"uid": uid, "conn_id": msg.self_id}}).to_string());
                            let mut sessions_guard = self.sessions_lock.write().unwrap();
                            let sessions = *sessions_guard;
                            *sessions_guard = sessions + 1;
                        }
                        Sender::Admin => self.send_disconnect_standalone(
                            String::from("Vehicle with the specified ID has already connected."),
                            &msg.addr,
                            msg.self_id,
                            CloseCode::Policy,
                        ),
                        Sender::Pair(message) => {
                            self.message_vehicle(msg.room_id, message.clone());
                            self.send_disconnect_standalone(
                                message,
                                &msg.addr,
                                msg.self_id,
                                CloseCode::Normal,
                            );

                            // Issue #6
                            // self cannot be moved into a closure. Can't be cloned either
                            // So there is no way to make changes here properly and then send confirmation
                            // actix_web::rt::spawn(async {
                            //     // make db updates here
                            //     self.send_disconnect(&*message, &msg.self_id, CloseCode::Normal);
                            // });
                        }
                    }
                }
                Entry::Vacant(o) => match msg.sender {
                    Sender::Client(_) => self.send_disconnect_standalone(
                        String::from("Vehicle isn't active at the moment. Try again later."),
                        &msg.addr,
                        msg.self_id,
                        CloseCode::Protocol,
                    ),
                    Sender::Admin => {
                        let mut set = HashSet::new();
                        let mut admins = self.lock.write().unwrap();
                        admins.insert(msg.room_id.clone(), msg.self_id.clone());
                        drop(admins);
                        set.insert(msg.self_id.clone());
                        o.insert(set);
                        self.admins.insert(msg.room_id, msg.self_id.clone());
                        self.insert(msg.self_id, msg.addr, "");
                        let mut sessions_guard = self.sessions_lock.write().unwrap();
                        let sessions = *sessions_guard;
                        *sessions_guard = sessions + 1;
                    }
                    Sender::Pair(_) => self.send_disconnect_standalone(
                        String::from("Vehicle isn't active at the moment. Try again later."),
                        &msg.addr,
                        msg.self_id,
                        CloseCode::Protocol,
                    ),
                },
            }
        }
    }

    // handler for when a client sends messages
    // pending work here to handle requests appropriately
    impl Handler<ClientActorMessage> for Lobby {
        type Result = ();

        // echo the message back to all clients
        fn handle(&mut self, msg: ClientActorMessage, _: &mut SyncContext<Self>) -> Self::Result {
            match msg.mode {
                Mode::Broadcast => self.broadcast(msg.msg.to_string(), msg.room_id.clone(), msg.id),
                Mode::Whisper(target) => self.whisper(msg.msg.to_string(), msg.room_id, target),
                _ => self.message_vehicle(msg.room_id.clone(), msg.msg.to_string()),
            }
        }
    }
}
