#[path ="./vehicles.rs"]
mod vehicles;

use vehicles::Vehicle;

pub struct User {
    name: String,
    username: String,
    uid: String,
    email: String,
    vehicles: Vec<Vehicle>,
}

impl User {
    pub fn login() {

    }

    pub fn signup() {

    }
}