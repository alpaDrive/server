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