pub mod logger {
    use mongodb::{options::ClientOptions, Client, Database};

    pub struct Logger {
        database: Option<Database>,
    }

    impl Clone for Logger {
        fn clone(&self) -> Self {
            Logger {
                database: self.database.clone()
            }
        }
    }

    impl Default for Logger {
        fn default() -> Self {
            Logger { database: None }
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
            Logger { database: Some(database) }
        }
    }
}
