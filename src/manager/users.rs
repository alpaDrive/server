#[path ="./vehicles.rs"]
mod vehicles;

use vehicles::Vehicle;
use serde::{Deserialize, Serialize};
use serde_json::{Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub username: String,
    pub password: String,
    pub email: String,
    pub vehicles: Vec<Vehicle>,
}

impl User {
    pub fn parse_request(request: Value) -> User {
        let expected_fields = vec!["name", "username", "password", "email"];
        let mut values = Vec::new();

        for each in expected_fields {
            let value = match serde_json::from_value::<Value>(request[each].clone()) {
                Ok(data) => data.to_string(),
                Err(_) => "".to_string()
            };
            values.push(value);
        }

        User {
            name: values[0].clone(),
            username: values[1].clone(),
            password: values[2].clone(),
            email: values[3].clone(),
            vehicles: Vec::new()
        }
    }
}