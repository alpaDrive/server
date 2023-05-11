use chrono::{Datelike, Local, NaiveDateTime};
use core::fmt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::{ClientOptions, FindOneOptions},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Logger {
    database: Option<Database>,
    message_count_map: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    gear: Option<u32>,
    rpm: Option<u32>,
    speed: Option<u32>,
    location: Option<String>,
    odo: u32,
    stressed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Log {
    _id: Option<ObjectId>,
    date: NaiveDateTime,
    average_speed: u32,
    distance: u32,
    stress: u32,
    last_odometer: u32,
    message_count: u32,
    max_speed: (u32, String) 
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ gear: {:?}, rpm: {:?}, speed: {:?}, location: {:?}, odo: {}, stressed: {} }}",
            self.gear, self.rpm, self.speed, self.location, self.odo, self.stressed
        )
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Logger {
            database: self.database.clone(),
            message_count_map: self.message_count_map.clone(),
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger {
            database: None,
            message_count_map: HashMap::new(),
        }
    }
}

impl Logger {
    pub async fn new() -> Self {
        let mut client_options = ClientOptions::parse("mongodb://localhost:8080/")
            .await
            .unwrap();
        client_options.app_name = Some("alpadrive".to_string());
        let client = Client::with_options(client_options).unwrap();
        let database = client.database("alpadrive-logs");
        Logger {
            database: Some(database),
            message_count_map: HashMap::new(),
        }
    }

    // function extracts the last known document from the collection
    // if not present, then returns a (base stat, whether update required)
    async fn get_base_stats(&self, collection: Collection<Log>) -> (Log, bool) {
        let options = FindOneOptions::builder().sort(doc! { "_id": -1 }).build();
        let default = (
            Log {
                _id: Some(ObjectId::new()),
                average_speed: 0,
                distance: 0,
                last_odometer: 0,
                stress: 0,
                message_count: 0,
                date: Local::now().naive_local(),
                max_speed: (0, Local::now().format("%I:%M %p").to_string())
            },
            false,
        );

        match collection.find_one(None, options).await {
            Ok(value) => match value {
                Some(log) => {
                    // if the last inserted document is from a previous date then return default
                    let date = log.date;
                    let current_date = Local::now().naive_local();
                    if date.year() == current_date.year()
                        && date.month() == current_date.month()
                        && date.day() == current_date.day()
                    {
                        (log, true)
                    } else {
                        default
                    }
                }
                None => default,
            },
            Err(_) => default,
        }
    }

    pub async fn log(&mut self, message: Message, vid: String) {
        let collection = self
            .database
            .clone()
            .unwrap_or_else(|| panic!("Logger couldn't find an active database"))
            .collection::<Log>(&vid);
        let (mut base_stats, update_required) = self.get_base_stats(collection.clone()).await;

        if update_required {
            // perform calculation and update operations
            let id = match base_stats._id {
                Some(value) => value,
                None => ObjectId::new(),
            };
            let distance = message.odo - base_stats.last_odometer;
            base_stats.distance += distance;
            let mut count = base_stats.message_count;

            if let Some(speed) = message.speed {
                if speed > base_stats.max_speed.0 {
                    base_stats.max_speed = (speed, Local::now().format("%I:%M %p").to_string())
                }
                base_stats.average_speed =
                    ((base_stats.average_speed * (count)) + speed) / ((count) + 1);
                count += 1;
            }

            if message.stressed {
                base_stats.stress = ((base_stats.stress * (count - 1)) + 1) / (count);
                count += 1;
            }

            base_stats.message_count = count;
            base_stats.last_odometer = message.odo;

            match collection
                .update_one(
                    doc! {"_id": id},
                    doc! {
                        "$set": {
                            "average_speed": base_stats.average_speed,
                            "distance": base_stats.distance,
                            "stress": base_stats.stress,
                            "last_odometer": base_stats.last_odometer,
                            "message_count": base_stats.message_count,
                            "max_speed": [base_stats.max_speed.0, base_stats.max_speed.1]
                        }
                    },
                    None,
                )
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    println!("{:?}", e);
                }
            };
        } else {
            // just insert the data as a new document
            base_stats.last_odometer = message.odo;
            let speed = message.speed.unwrap_or(0);
            base_stats.average_speed = speed;
            base_stats.max_speed = (speed, Local::now().format("%I:%M %p").to_string());
            collection
                .insert_one(&base_stats, None)
                .await
                .expect("Failed to insert document");
        }
    }
}
