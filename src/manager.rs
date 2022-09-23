mod users;
mod vehicles;

use users::User;
use vehicles::Vehicle;
use mongodb::sync::Client;

pub struct Manager {
    mongodb: Client,
    active_users: Vec<User>,
    active_vehicles: Vec<Vehicle>,
}

impl Manager {
    pub fn start() -> Manager {
        let active_users: Vec<User> = Vec::new();
        let active_vehicles: Vec<Vehicle> = Vec::new();

        let client = Client::with_uri_str("mongodb://localhost:27017").unwrap();

        Manager {
            mongodb: client,
            active_users: active_users,
            active_vehicles: active_vehicles
        }
    }

    pub fn login(&self, username: String, password: String) {
        
    }
}