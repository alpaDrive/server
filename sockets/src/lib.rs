mod messages;

pub mod sockets {
    use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
    use actix::prelude::{Actor, Context, Handler, Recipient};
    use std::collections::{HashMap, HashSet};
    use uuid::Uuid;

    pub struct Lobby {
        sessions: HashMap<Uuid, Recipient<WsMessage>>,
        rooms: HashMap<Uuid, HashSet<Uuid>>,      //room id  to list of users id
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
        fn send_message(&self, message: &str, id_to: &Uuid) {
            if let Some(socket_recipient) = self.sessions.get(id_to) {
                let _ = socket_recipient
                    .do_send(WsMessage(message.to_owned()));
            } else {
                println!("attempting to send message but couldn't find user id.");
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
            // create a room if necessary, and then add the id to it
            self.rooms
                .entry(msg.lobby_id)
                .or_insert_with(HashSet::new).insert(msg.self_id);

            // send to everyone in the room that new uuid just joined
            self
                .rooms
                .get(&msg.lobby_id)
                .unwrap()
                .iter()
                .filter(|conn_id| *conn_id.to_owned() != msg.self_id)
                .for_each(|conn_id| self.send_message(&format!("{} just joined!", msg.self_id), conn_id));

            // store the address
            self.sessions.insert(
                msg.self_id,
                msg.addr,
            );

            // send self your new uuid
            self.send_message(&format!("your id is {}", msg.self_id), &msg.self_id);
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