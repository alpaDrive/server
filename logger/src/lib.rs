use chrono::{Datelike, Local, NaiveDateTime};
use mongodb::{
    bson::doc,
    options::{ClientOptions, FindOneOptions},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json;

pub struct Logger {
    database: Option<Database>,
    message_count_map: HashMap<String, u32>
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    gear: Option<u32>,
    rpm: Option<u32>,
    speed: Option<u32>,
    odo: u32,
    stressed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Log {
    date: NaiveDateTime,
    average_speed: u32,
    distance: u32,
    stress: u32,
    last_odometer: u32,
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Logger {
            database: self.database.clone(),
            message_count_map: self.message_count_map.clone()
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger { database: None, message_count_map: HashMap::new() }
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
            message_count_map: HashMap::new()
        }
    }

    // function extracts the last known document from the collection
    // if not present, then returns a (base stat, whether update required)
    async fn get_base_stats(&self, collection: Collection<Log>) -> (Log, bool) {
        let options = FindOneOptions::builder().sort(doc! { "_id": -1 }).build();
        let default = (
            Log {
                average_speed: 0,
                distance: 0,
                last_odometer: 0,
                stress: 0,
                date: Local::now().naive_local(),
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

    async fn log(&mut self, message: Message, vid: String) {
        let collection = self
            .database
            .clone()
            .unwrap_or_else(|| panic!("Logger couldn't find an active database"))
            .collection::<Log>(&vid);
        let (mut base_stats, update_required) = self.get_base_stats(collection).await;

        if update_required {
            // perform calculation and update operations
            let distance = message.odo - base_stats.last_odometer;
            base_stats.distance += distance;

            let message_count = self.message_count_map.entry(vid.clone()).or_insert(0);

            if let Some(speed) = message.speed {
                base_stats.average_speed = ((base_stats.average_speed * (*message_count)) + speed) / ((*message_count) + 1);
                *message_count += 1;
            }
            
            if !message.stressed {
                base_stats.stress = ((base_stats.stress * (*message_count - 1)) + 1) / (*message_count);
                *message_count += 1;
            }            

            base_stats.last_odometer = message.odo;

            // collection
            //     .insert_one(
            //         doc! { "distance": base_stats.distance,
            //         "average_speed": base_stats.average_speed,
            //         "stress": base_stats.stress,
            //         "date": base_stats.date,
            //         "last_odometer": base_stats.last_odometer},
            //         None,
            //     )
            //     .await
            //     .expect("Failed to insert document");
        } else {
            // just insert the data as a new document
            base_stats.last_odometer = message.odo;
            let speed = message.speed.unwrap_or(0);
            base_stats.average_speed = speed;

            // collection
            //     .insert_one(
            //         doc! {
            //             "distance": base_stats.distance,
            //             "average_speed": base_stats.average_speed,
            //             "stress": base_stats.stress,
            //             "date": base_stats.date,
            //             "last_odometer": base_stats.last_odometer
            //         },
            //         None,
            //     )
            //     .await
            //     .expect("Failed to insert document");
        }
    }
}
