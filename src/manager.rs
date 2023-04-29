extern crate sockets;
extern crate types;

use actix::Addr;
use actix_web::{web::Payload, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};
use serde_json::{json, Value};
use sockets::{
    sockets::Lobby,
    ws::{Sender, WsConn},
};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use sysinfo::SystemExt;
use types::actors::{users::User, vehicles::Vehicle};
use uuid::Uuid;

pub struct Manager {
    db: Database,
    lobby: Addr<Lobby>,
    admins: Arc<RwLock<HashMap<String, String>>>,
    sessions: Arc<RwLock<usize>>
}

impl Manager {
    pub fn start(database: Database, lobby: Addr<Lobby>, admins: Arc<RwLock<HashMap<String, String>>>, sessions: Arc<RwLock<usize>>) -> Manager {
        Manager {
            db: database,
            lobby,
            admins,
            sessions
        }
    }

    pub fn status(&self, data: Value) -> HttpResponse {
        let response = match serde_json::from_value(data["systemstat"].clone()) {
            Ok(data) => {
                let vehicles = self.admins.read().unwrap().len();
                let sessions = self.sessions.read().unwrap();
                let response = match data {
                    true => {
                        let mut system = sysinfo::System::new();
                        system.refresh_all();
                        json!({
                            "active_users": *sessions - vehicles,
                            "active_vehicles": vehicles,
                            "active_sessions": *sessions,
                            "memory_available": format!("{:.2} GB", system.get_total_memory() as f64 * 0.000001),
                            "memory_used": format!("{:.2} GB", system.get_used_memory() as f64 * 0.000001),
                            "total_swap": format!("{:.2} GB", system.get_total_swap() as f64 * 0.000001),
                            "swap_used": format!("{:.2} GB", system.get_used_swap() as f64 * 0.000001)
                        }).to_string()
                    },
                    false => json!({
                        "active_users": *sessions - vehicles,
                        "active_vehicles": vehicles,
                        "active_sessions": *sessions
                    }).to_string()
                };
                HttpResponse::Ok().body(response)
            },
            Err(_) => HttpResponse::BadRequest().body(json!({"error": "Request is not in a supported format. Make sure you have included the flag for systemstat."}).to_string())
        };
        response
    }

    async fn is_vehicle_used_by_user(&self, vehicle_id: ObjectId) -> mongodb::error::Result<bool> {
        let query = doc! {
            "vehicles": {
                "$elemMatch": {
                    "$eq": vehicle_id
                }
            }
        };
    
        let collection = self.db.collection::<User>("users");
        let result = collection.count_documents(query, None).await?;
    
        Ok(result > 0)
    }

    // Lobby management

    pub async fn joinvehicle(
        &self,
        uid: String,
        request: &HttpRequest,
        stream: Payload,
    ) -> HttpResponse {
        let collection = self.db.collection::<Vehicle>("vehicles");
        match collection.find_one(doc! {"_id": ObjectId::from_str(&uid.to_string().replace('"', "")).unwrap()}, None).await {
                Ok(data) => match data {
                    Some(data) => {
                        let ws = WsConn::new(data._id.to_hex(), Uuid::new_v4().to_string(), self.lobby.clone(), Sender::Admin);
                        match ws::start(ws, request, stream) {
                            Ok(response) => response,
                            Err(e) => HttpResponse::InternalServerError().body(json!({"error": "The server faced an internal error trying to create a room.", "stacktrace": format!("{:#?}", e)}).to_string())
                        }
                    },
                    None => HttpResponse::NotFound().body(json!({"error": "There is no vehicle with the supplied ID. Consider registering it first at /vehicle/register."}).to_string())          
                },
                Err(_) => HttpResponse::InternalServerError().body(json!({"error": "The server had an error trying to execute mongodb::Collection.insert_one()"}).to_string())
            }
    }

    pub async fn joinuser(
        &self,
        uid: String,
        vid: String,
        request: &HttpRequest,
        stream: Payload,
    ) -> HttpResponse {
        let users = self.db.collection::<User>("users");
        let vehicles = self.db.collection::<Vehicle>("vehicles");
        match users.find_one(doc! {"_id": ObjectId::from_str(&uid.to_string().replace('"', "")).unwrap()}, None).await {
            Ok(res) => match res {
                Some(user) => {
                   match vehicles.find_one(doc! {"_id": ObjectId::from_str(&vid.to_string().replace('"', "")).unwrap()}, None).await {
                        Ok(res) => match res {
                            Some(vehicle) => {
                                if user.vehicles.contains(&vehicle._id) {
                                    let ws = WsConn::new(vid, Uuid::new_v4().to_string(), self.lobby.clone(), Sender::Client(uid));
                                    let response = match ws::start(ws, request, stream) {
                                        Ok(response) => response,
                                        Err(e) => HttpResponse::InternalServerError().body(json!({"error": "The server faced an internal error trying to create a room.", "stacktrace": format!("{:#?}", e)}).to_string())
                                    };
                                    response
                                }
                                else {
                                    HttpResponse::Unauthorized().body(json!({"error": "This user has no access to the vehicle. Securely link it first."}).to_string())
                                }
                            },
                            None => HttpResponse::NotFound().body(json!({"error": "There is no vehicle with the supplied ID. Consider registering it first."}).to_string())
                        },
                        Err(_) => HttpResponse::InternalServerError().body(json!({"error": "The server had an error trying to execute mongodb::Collection.find_one()"}).to_string())
                   }
                },
                None => HttpResponse::NotFound().body(json!({"error": "There is no user with the supplied ID. Consider signing up first."}).to_string())          
            },
            Err(_) => HttpResponse::InternalServerError().body(json!({"error": "The server had an error trying to execute mongodb::Collection.find_one()"}).to_string())
        }
    }

    // Account management

    pub async fn signup(&self, request: Value) -> HttpResponse {
        let user = User::parse_request(request);
        let collection = self.db.collection::<User>("users");
        // this processing can be further sped up by directly using doc!{"$or": [{"username": user.username}, {"email": user.email}]}
        // but it would take away from the amount of detail which can be provided to the client
        // compromising on it currently as sign up is a one time operation
        match collection.find_one(doc! {"email": &user.email}, None).await {
            Ok(data) => match data {
                Some(data) => HttpResponse::Conflict().body(
                    json!({
                        "error":
                            format!("Another user already exists with the email {}", data.email)
                    })
                    .to_string(),
                ),
                None => match collection
                    .find_one(doc! {"username": &user.username}, None)
                    .await
                {
                    Ok(data) => match data {
                        Some(data) => HttpResponse::Conflict().body(
                            json!({
                                "error":
                                    format!(
                                        "Another user already exists with the username {}",
                                        data.username
                                    )
                            })
                            .to_string(),
                        ),
                        None => {
                            match collection.insert_one(User {
                                _id: Some(ObjectId::new()),
                                name: user.name,
                                username: user.username,
                                password: user.password,
                                email: user.email,
                                vehicles: Vec::new()
                            }, None).await {
                                Ok(data) => HttpResponse::Ok().body(json!({"success": "Successfully signed up user", "uid": data.inserted_id}).to_string()),
                                Err(_) => HttpResponse::InternalServerError().body(json!({"error": "The server had an error trying to execute mongodb::Collection.insert_one()"}).to_string())
                            }
                        }
                    },
                    Err(_) => HttpResponse::InternalServerError()
                        .body("Error when trying to execute mongodb::Collection.find_one()"),
                },
            },
            Err(_) => HttpResponse::InternalServerError()
                .body("Error when trying to execute mongodb::Collection.find_one()"),
        }
    }

    pub async fn login(&self, request: Value) -> HttpResponse {
        let user = User::parse_request(request);

        let collection = self.db.collection::<User>("users");
        match collection.find_one(doc!{"$or": [{"username": user.username}, {"email": user.email}]}, None).await {
            Ok(data) => match data {
                Some(data) => {
                    if data.password != user.password { HttpResponse::Unauthorized().body(json!({"error": "Wrong credentials"}).to_string()) }
                    else {
                        let mut vehicles: Vec<Vehicle> = vec![]; 
                        if let Ok(mut cursor) = self.db.collection::<Vehicle>("vehicles").find(doc! {"_id": {"$in": data.vehicles}}, None).await {
                            let mut flag = true;
                            while flag {
                                if let Ok(remains) = cursor.advance().await {
                                    if !remains { flag = false; }
                                    else {
                                        if let Ok(vehicle) = cursor.deserialize_current() { vehicles.push(vehicle) }
                                    }
                                }
                            }
                        };
                        HttpResponse::Ok().body(json!({
                            "uid": match data._id {
                                Some(id) => id,
                                None => ObjectId::new()
                            },
                            "name": data.name,
                            "username": data.username,
                            "email": data.email,
                            "vehicles": vehicles
                        }).to_string())
                     }
                },
                None => HttpResponse::NotFound().body(json!({"error": "User with this username wasn't found on this server"}).to_string())
            },
            Err(_) => HttpResponse::InternalServerError().body(json!({"error": "There was an error when trying to execute mongodb::collection.find_one()"}).to_string())
        }
    }

    pub async fn refreshvehicles(&self, request: Value) -> HttpResponse {
        let id = User::parse_id(request);
        match self.db.collection::<User>("users").find_one(doc! {"_id": id}, None).await {
            Ok(user) => match user {
                Some(user) => {
                    let mut vehicles: Vec<Vehicle> = vec![]; 
                        if let Ok(mut cursor) = self.db.collection::<Vehicle>("vehicles").find(doc! {"_id": {"$in": user.vehicles}}, None).await {
                            let mut flag = true;
                            while flag {
                                if let Ok(remains) = cursor.advance().await {
                                    if !remains { flag = false; }
                                    else {
                                        if let Ok(vehicle) = cursor.deserialize_current() { vehicles.push(vehicle) }
                                    }
                                }
                            }
                        };
                    HttpResponse::Ok().body(json!({
                        "count": vehicles.len(),
                        "vehicles": vehicles
                    }).to_string())
                },
                None => HttpResponse::NotFound().body(json!({"error": "User with this ID wasn't found on this server"}).to_string())
            },
            Err(_) => HttpResponse::InternalServerError().body(json!({"error": "There was an error when trying to execute mongodb::collection.find_one()"}).to_string())
        }
    }

    pub async fn registervehicle(&self, request: Value) -> HttpResponse {
        let vehicle = Vehicle::parse_request(request);
        let collection = self.db.collection::<Vehicle>("vehicles");
        match collection.insert_one(vehicle, None).await {
                Ok(data) => HttpResponse::Ok().body(json!({"success": "Vehicle was registered", "id": data.inserted_id}).to_string()),
                Err(_) => HttpResponse::InternalServerError().body(json!({"error": "There was an error trying to execute mongodb::collection.insert_one()"}).to_string())
            }
    }

    pub async fn pair(&self, uid: String, vid: String, request: &HttpRequest, stream: Payload, initial: bool) -> HttpResponse {
        let users = self.db.collection::<User>("users");
        let vehicles = self.db.collection::<Vehicle>("vehicles");

        match users.find_one(doc! {"_id": ObjectId::from_str(&uid.to_string().replace('"', "")).unwrap()}, None).await {
            Ok(result) => {
                match result {
                    Some(user) => {
                        match vehicles.find_one(doc! {"_id": ObjectId::from_str(&vid.to_string().replace('"', "")).unwrap()}, None).await {
                            Ok(result) => {
                                let auth = match self.is_vehicle_used_by_user(ObjectId::from_str(&vid.to_string().replace('"', "")).unwrap()).await {
                                    Ok(result) => result,
                                    Err(_) => true
                                };
                                // if this is the first time but it's already been paired
                                if initial && auth {
                                    HttpResponse::Unauthorized().body(json!({"error": "This code has expired", "suggestion": "Use the code generated by the app."}).to_string())
                                } else {
                                    match result {
                                        Some(vehicle) => {
                                            let mut list = user.vehicles.clone();
                                            list.insert(00, vehicle._id);
                                            let message = match users.update_one(user.document(), doc! {"$set": {"vehicles": list}}, None).await {
                                                Ok(result) => {
                                                    if result.modified_count > 0 { String::from("Pair successful") }
                                                    else { String::from("Database had an unknown error") }
                                                },
                                                Err(e) => format!("Database reported an error: {:#?}", e)
                                            };
                                            let ws = WsConn::new(vid.clone(), Uuid::new_v4().to_string(), self.lobby.clone(), Sender::Pair(json!({"message": message.clone(), "uid": uid.clone(), "vid": vid.clone()}).to_string()));
                                            match ws::start(ws, request, stream) {
                                                Ok(response) => response,
                                                Err(e) => HttpResponse::InternalServerError().body(json!({"error": "The server faced an internal error trying to create a room.", "stacktrace": format!("{:#?}", e)}).to_string())
                                            }
                                        },
                                        None => HttpResponse::NotFound().body(json!({"error": "There is no vehicle with the specified ID.", "suggestion": "Register the vehicle at /vehicle/register"}).to_string())
                                    }
                                }
                            },
                            Err(e) => HttpResponse::InternalServerError().body(json!({"error": "There was an error trying to execute mongodb::collection.find_one()", "stacktrace": format!("{:#?}", e)}).to_string())
                        }
                    },
                    None => HttpResponse::NotFound().body(json!({"error": "There is no user with the specified ID.", "suggestion": "Sign up the user at /signup"}).to_string())
                }
            },
            Err(e) => HttpResponse::InternalServerError().body(json!({"error": "There was an error trying to execute mongodb::collection.find_one()", "stacktrace": format!("{:#?}", e)}).to_string())
        }
    }
}
