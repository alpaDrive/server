extern crate types;

use actix_web::HttpResponse;
use mongodb::bson::oid::ObjectId;
use mongodb::{bson::doc, Database};
use serde_json::{json, Value};
use sysinfo::SystemExt;

use types::actors::{users::User, vehicles::Vehicle};

pub struct Manager {
    db: Database,
    active_users: Vec<User>,
    active_vehicles: Vec<Vehicle>,
}

impl Manager {
    pub fn start(database: Database) -> Manager {
        let active_users: Vec<User> = Vec::new();
        let active_vehicles: Vec<Vehicle> = Vec::new();
        Manager {
            db: database,
            active_users: active_users,
            active_vehicles: active_vehicles,
        }
    }

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
                    else { HttpResponse::Ok().body(json!({
                        "uid": match data._id {
                            Some(id) => id,
                            None => ObjectId::new()
                        },
                        "name": data.name,
                        "username": data.username,
                        "email": data.email,
                        "vehicles": data.vehicles
                    }).to_string()) }
                },
                None => HttpResponse::NotFound().body(json!({"error": "User with this username wasn't found on this server"}).to_string())
            },
            Err(_) => HttpResponse::InternalServerError().body(json!({"error": "There was an error when trying to execute mongodb::collection.find_one()"}).to_string())
        }
    }

    pub fn status(&self, data: Value) -> HttpResponse {
        let response = match serde_json::from_value(data["systemstat"].clone()) {
            Ok(data) => {
                let response = match data {
                    true => {
                        let mut system = sysinfo::System::new();
                        system.refresh_all();
                        json!({
                            "active_users": self.active_users.len(),
                            "active_vehicles": self.active_vehicles.len(),
                            "memory_available": format!("{:.2} GB", system.get_total_memory() as f64 * 0.000001),
                            "memory_used": format!("{:.2} GB", system.get_used_memory() as f64 * 0.000001),
                            "total_swap": format!("{:.2} GB", system.get_total_swap() as f64 * 0.000001),
                            "swap_used": format!("{:.2} GB", system.get_used_swap() as f64 * 0.000001)
                        }).to_string()
                    },
                    false => json!({
                        "active_users": self.active_users.len(),
                        "active_vehicles": self.active_vehicles.len(),
                    }).to_string()
                };
                HttpResponse::Ok().body(response)
            },
            Err(_) => HttpResponse::NoContent().body(json!({"error": "Request is not in a supported format. Make sure you have included the flag for systemstat."}).to_string())
        };
        response
    }

    pub async fn registervehicle(&self, request: Value) -> HttpResponse {
        let vehicle = Vehicle::parse_request(request);
        let collection = self.db.collection::<Vehicle>("vehicles");
        match collection.insert_one(vehicle, None).await {
            Ok(data) => HttpResponse::Ok().body(json!({"success": "Vehicle was registered", "id": data.inserted_id}).to_string()),
            Err(_) => HttpResponse::InternalServerError().body(json!({"error": "There was an error trying to execute mongodb::collection.insert_one()"}).to_string())
        }
    }

    pub fn echo(&self) {
        println!("I'm working")
    }
}
