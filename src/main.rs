mod manager;

pub use crate::manager::Manager;
use actix_web::{web, get, post, App, HttpResponse, HttpServer, Responder};
use mongodb::{Client, options::ClientOptions};
use serde_json::json;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/login")]
async fn login(context: web::Data<Manager>, req_body: String) -> impl Responder {
    let response: HttpResponse = match serde_json::from_str(&req_body) {
        Ok(data) => context.login(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    };
    response
}

#[get("/status")]
async fn status(context: web::Data<Manager>, req_body: String) -> impl Responder {
    let response: HttpResponse = match serde_json::from_str(&req_body) {
            Ok(data) => context.status(data),
            Err(_) => HttpResponse::NotAcceptable().body(json!({
                "error": "Failed to parse request. Make sure it is a valid JSON payload."
            }).to_string())
        };
    response
}

#[post("/signup")]
async fn signup(context: web::Data<Manager>, req_body: String) -> impl Responder {
     let response: HttpResponse = match serde_json::from_str(&req_body) {
        Ok(data) => context.signup(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    };
    response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:8080/").await.unwrap();

    client_options.app_name = Some("alpadrive".to_string());

    let client = Client::with_options(client_options).unwrap();
    let database = client.database("alpadrive");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Manager::start(database.clone())))
            .service(hello)
            .service(login)
            .service(status)
            .service(signup)
    })
    .bind(("127.0.0.1", 7878))?
    .run()
    .await
}