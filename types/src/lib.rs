pub mod actors {
    pub mod vehicles {
        use mongodb::bson::oid::ObjectId;
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        #[derive(Debug, Serialize, Deserialize)]
        pub struct Vehicle {
            pub _id: ObjectId,
            pub company: String,
            pub model: String
        }

        impl Vehicle {
            pub fn parse_request(request: Value) -> Vehicle {
                let expected_fields = vec!["model", "company"];
                let mut values = Vec::new();

                for each in expected_fields {
                    let value = match serde_json::from_value::<Value>(request[each].clone()) {
                        Ok(data) => data.to_string(),
                        Err(_) => "".to_string()
                    };
                    values.push(value);
                }

                Vehicle { _id: ObjectId::new(), company: values[1].clone(), model: values[0].clone() }
            }
        }
    }

    pub mod users {
        use super::vehicles::Vehicle;
        use mongodb::bson::oid::ObjectId;
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        #[derive(Debug, Serialize, Deserialize)]
        pub struct User {
            pub _id: Option<ObjectId>,
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
                    _id: None,
                    name: values[0].clone(),
                    username: values[1].clone(),
                    password: values[2].clone(),
                    email: values[3].clone(),
                    vehicles: Vec::new()
                }
            }
        }
    }
}