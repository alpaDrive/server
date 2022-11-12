pub mod messages;
pub mod ws;

pub mod sockets {
    use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage, Action};
    use crate::ws::Mode;
    use actix::prelude::{Actor, Context, Handler, Recipient};
    use actix_web_actors::ws::CloseCode;
    use std::collections::{HashMap, HashSet};
    use std::collections::hash_map::Entry;
    
    pub struct Lobby {
        sessions: HashMap<String, Recipient<WsMessage>>,
        rooms: HashMap<String, HashSet<String>>,      //room id  to list of users id
    }

    impl Default for Lobby {
        fn default() -> Lobby {
            Lobby {
                sessions: HashMap::new(),
                rooms: HashMap::new(),
            }
        }
    }

    impl Lobby {
        
        // send message to user in the room
        fn send_message(&self, message: &str, id_to: &String) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient
                    .do_send(WsMessage {
                        message: message.to_string(),
                        id: id_to.to_string(),
                        action: Action::Send
                    });
            }
        }
        // send disconnect to an existing user in the room
        fn send_disconnect(&self, reason: &str, id_to: &String, code: CloseCode) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient
                    .do_send(WsMessage {
                        message: reason.to_string(),
                        id: id_to.to_string(),
                        action: Action::Disconnect(code)
                    });
            }
        }
        // disconnect user before entering the room (special cases liek pair)
        fn send_disconnect_standalone(&self, reason: String, recipient: &Recipient<WsMessage>, id_to: String, code: CloseCode) {
            let _ = recipient.do_send(WsMessage {
                message: reason,
                id: id_to,
                action: Action::Disconnect(code)
            });
        }
        // helper to return the id of the vehicle in a room
        fn get_vehicle_id(&mut self, room: String) -> Option<&String> {
            if let Entry::Occupied(o) = self.rooms.entry(room) {
                let mut temp = o.into_mut().iter().take(1);
                temp.next()
            } else { None }
        }
        // send a message to the admin vehicle
        fn message_vehicle(&mut self, room: String, message: String) {
            let sessions = self.sessions.clone();
            if let Some(room) = self.get_vehicle_id(room) {
                if let Some(socket_recipient) = sessions.get(room) {
                    let _ = socket_recipient
                        .do_send(WsMessage {
                            message: message,
                            id: room.to_string(),
                            action: Action::Send
                        });
                }
            }
        }
        // add an actor to the sessions map
        fn insert(&mut self, self_id: String, addr: Recipient<WsMessage>) {
            self.sessions.insert(
                self_id.clone(),
                addr.clone(),
            );
            self.send_message(&"Connected", &self_id);
        }
    }

    impl Actor for Lobby {
        type Context = Context<Self>;
    }

    // Handler for Disconnect message.
    impl Handler<Disconnect> for Lobby {
        type Result = ();

        fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
            if self.sessions.remove(&msg.id).is_some() {
                self.rooms
                    .get(&msg.room_id)
                    .unwrap()
                    .iter()
                    .filter(|conn_id| *conn_id.to_owned() != msg.id)
                    .for_each(|user_id| self.send_message(&format!("{} disconnected.", &msg.id), user_id));
                if let Some(lobby) = self.rooms.get_mut(&msg.room_id) {
                    if lobby.len() > 1 {
                        lobby.remove(&msg.id);
                    } else {
                        //only one in the lobby, remove it entirely
                        self.rooms.remove(&msg.room_id);
                    }
                }
            }
        }
    }

    // handler for connect message
    // pending work here to manage room creation/joining appropriately
    impl Handler<Connect> for Lobby {
        type Result = ();

        fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
            // if it's a vehicle issuing connect
            match self.rooms.entry(msg.room_id.clone()) {
                Entry::Occupied(mut o) => {
                    match msg.mode {
                        Mode::Client => {
                            o.get_mut().insert(msg.self_id.clone());
                            self.insert(msg.self_id, msg.addr);
                        },
                        Mode::Admin => self.send_disconnect_standalone("Vehicle with the specified ID has already connected.".to_string(), &msg.addr, msg.self_id, CloseCode::Policy),
                        Mode::Pair(message) => {
                            self.message_vehicle(msg.room_id, message.clone());
                            self.send_disconnect_standalone(message, &msg.addr, msg.self_id, CloseCode::Normal);

                            // Issue #6
                            // self cannot be moved into a closure. Can't be cloned either
                            // So there is no way to make changes here properly and then send confirmation
                            // actix_web::rt::spawn(async {
                            //     // make db updates here
                            //     self.send_disconnect(&*message, &msg.self_id, CloseCode::Normal);
                            // });
                        }
                    }
                },
                Entry::Vacant(o) => {
                    match msg.mode {
                        Mode::Client => self.send_disconnect_standalone(String::from("Vehicle isn't active at the moment. Try again later."), &msg.addr, msg.self_id, CloseCode::Protocol),
                        Mode::Admin => {
                            let mut set = HashSet::new();
                            set.insert(msg.self_id.clone());
                            o.insert(set);
                            self.insert(msg.self_id, msg.addr);
                        },
                        Mode::Pair(_) => self.send_disconnect_standalone(String::from("Vehicle isn't active at the moment. Try again later."), &msg.addr, msg.self_id, CloseCode::Protocol)
                    }
                }

            }
        }
    }


    // handler for when a client sends messages
    // pending work here to handle requests appropriately
    impl Handler<ClientActorMessage> for Lobby {
        type Result = ();

        // echo the message back to all clients
        fn handle(&mut self, msg: ClientActorMessage, _: &mut Context<Self>) -> Self::Result {
            self.rooms.get(&msg.room_id).unwrap().iter().for_each(|client| self.send_message(&msg.msg, client));
        }
    }
}