use chrono::{Datelike, Local, FixedOffset};
use core::fmt;
use futures_util::stream::StreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::{ClientOptions, FindOneOptions, FindOptions},
    Client, Collection, Database,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

pub struct Logger {
    database: Option<Database>,
    message_count_map: HashMap<String, u32>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Location {
    latitude: f64,
    longitude: f64
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    gear: Option<u32>,
    rpm: Option<u32>,
    speed: Option<u32>,
    location: Option<Location>,
    temp: Option<u32>,
    fuel: Option<u32>,
    odo: u32,
    stressed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Log {
    _id: Option<ObjectId>,
    date: String,
    average_speed: u32,
    distance: u32,
    stress: u32,
    last_odometer: u32,
    message_count: u32,
    max_speed: (u32, String),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ gear: {:?}, rpm: {:?}, speed: {:?}, odo: {}, stressed: {} }}",
            self.gear, self.rpm, self.speed, self.odo, self.stressed
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
        let today = Local::now().naive_local();
        let default = (
            Log {
                _id: Some(ObjectId::new()),
                average_speed: 0,
                distance: 0,
                last_odometer: 0,
                stress: 0,
                message_count: 0,
                date: format!("{}-{}-{}", today.day(), today.month(), today.year()),
                max_speed: (0, Local::now().format("%I:%M %p").to_string()),
            },
            false,
        );

        match collection.find_one(None, options).await {
            Ok(value) => match value {
                Some(log) => {
                    // if the last inserted document is from a previous date then return default
                    let date = log.clone().date;
                    let current_date = Local::now().naive_local();
                    if date
                        == format!(
                            "{}-{}-{}",
                            current_date.day(),
                            current_date.month(),
                            current_date.year()
                        )
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

    fn calculate_degradation(&self, events: u32) -> f64 {
        let required_events = 1000.0;
        (events as f64 / required_events) * 0.01
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

            if message.odo > 0  {
                let distance = message.odo - base_stats.last_odometer;
                base_stats.distance += distance;
            }
            let mut count = base_stats.message_count;

            if let Some(speed) = message.speed {
                if speed > base_stats.max_speed.0 {
                    base_stats.max_speed = (speed, Local::now().with_timezone(&FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap()).format("%I:%M %p").to_string())
                }

                if base_stats.average_speed > 0 {
                    base_stats.average_speed =
                    ((base_stats.average_speed * (count)) + speed) / ((count) + 1);
                } else { base_stats.average_speed = speed; }

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

    pub async fn dailylogs(&self, date: String, vid: String) -> Result<String, String> {
        let collection = self
            .database
            .clone()
            .unwrap_or_else(|| panic!("Logger couldn't find an active database"))
            .collection::<Log>(&vid);
        let filter = doc! {"date": date};
        let options = FindOneOptions::builder().build();

        match collection.find_one(filter, options).await {
            Ok(result) => match result {
                Some(result) => Ok(json!({
                    "average_speed": result.average_speed,
                    "stress_count": result.stress,
                    "degradation": self.calculate_degradation(result.stress),
                    "distance_travelled": result.distance,
                    "last_odometer": result.last_odometer,
                    "max_speed": {
                        "speed": result.max_speed.0,
                        "hit_at": result.max_speed.1
                    }
                })
                .to_string()),
                None => Err(String::from("No results were found for this day.")),
            },
            Err(_) => Err(String::from("Some bad error occured")),
        }
    }

    pub async fn periodiclogs(
        &self,
        vid: String,
        start_date: String,
        end_date: String,
    ) -> Result<String, String> {
        let collection = self
            .database
            .clone()
            .unwrap_or_else(|| panic!("Logger couldn't find active database"))
            .collection::<Log>(&vid);
        let filter = doc! {"date": {
            "$gte": start_date,
            "$lte": end_date,
        }};

        match collection
            .find(filter, FindOptions::builder().build())
            .await
        {
            Ok(mut cursor) => {
                let mut average_speed = 0;
                let mut distance = 0;
                let mut max_speed = (0, String::from(""));
                let mut last_odo = 0;
                let mut stress_count = 0;
                let mut degradation = 0.0;
                let mut length = 0;

                while let Some(result) = cursor.next().await {
                    if let Ok(doc) = result {
                        distance += doc.distance;
                        average_speed += doc.average_speed;
                        last_odo = doc.last_odometer;
                        stress_count += doc.stress;
                        degradation += self.calculate_degradation(doc.stress);
                        length += 1;
                        if max_speed.0 < doc.max_speed.0 {
                            max_speed = doc.max_speed;
                        }
                    }
                }
                degradation /= length as f64;

                Ok(json!({
                    "distance_travelled": distance,
                    "average_speed": average_speed/length,
                    "stress_count": stress_count,
                    "degradation": degradation,
                    "last_odometer": last_odo,
                    "max_speed": {
                        "speed": max_speed.0,
                        "hit_at": max_speed.1
                    }
                })
                .to_string())
            }
            Err(_) => Err(String::from("An unexpected error occured")),
        }
    }

    pub async fn overall_logs(&self, vid: String) -> Result<String, String> {
        let collection = self
            .database
            .clone()
            .unwrap_or_else(|| panic!("Logger couldn't find active database"))
            .collection::<Log>(&vid);

        match collection.find(None, None).await {
            Ok(mut cursor) => {
                let mut average_speed = 0;
                let mut distance = 0;
                let mut max_speed = (0, String::from(""));
                let mut last_odo = 0;
                let mut stress_count = 0;
                let mut degradation = 0.0;
                let mut length = 0;

                while let Some(result) = cursor.next().await {
                    if let Ok(doc) = result {
                        distance += doc.distance;
                        average_speed += doc.average_speed;
                        last_odo = doc.last_odometer;
                        stress_count += doc.stress;
                        degradation += self.calculate_degradation(doc.stress);
                        length += 1;
                        if max_speed.0 < doc.max_speed.0 {
                            max_speed = doc.max_speed;
                        }
                    }
                }
                degradation /= length as f64;

                Ok(json!({
                    "distance_travelled": distance,
                    "average_speed": average_speed/length,
                    "stress_count": stress_count,
                    "degradation": degradation,
                    "last_odometer": last_odo,
                    "max_speed": {
                        "speed": max_speed.0,
                        "hit_at": max_speed.1
                    }
                })
                .to_string())
            }
            Err(_) => Err(String::from("An unexpected error occured")),
        }
    }
}
