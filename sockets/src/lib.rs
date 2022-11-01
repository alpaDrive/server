pub mod messages;
pub mod ws;

pub mod sockets {
    use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
    use actix::prelude::{Actor, Context, Handler, Recipient};
    use std::collections::{HashMap, HashSet};
    use std::collections::hash_map::Entry;
    use serde_json::json;

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
        
        // send message to client
        fn send_message(&self, message: &str, id_to: &String) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient
                    .do_send(WsMessage(json!({"message": message, "id": id_to})));
            }
        }
        fn send_disconnect(&self, reason: &str, id_to: &String) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient
                    .do_send(WsMessage(json!({"disconnect": reason, "id": id_to})));
            }
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
            if msg.isvehicle {
                match self.rooms.entry(msg.room_id) {
                    Entry::Occupied(_) => {
                        self.send_disconnect("Vehicle with the specified ID has already connected.", &msg.self_id);
                    },
                    Entry::Vacant(o) => {
                        let mut set = HashSet::new();
                        set.insert(msg.self_id.clone());
                        o.insert(set);

                        self.sessions.insert(
                            msg.self_id.clone(),
                            msg.addr.clone(),
                        );
                        self.send_message(&"Connected", &msg.self_id);
                    }
                }    
            } else {
                // user code goes here
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