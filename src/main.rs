mod manager;

use std::{sync::{Arc, RwLock}, collections::HashMap};

pub use crate::manager::Manager;
use actix_web::{web::{self, Path}, get, post, App, HttpResponse, HttpRequest, HttpServer, Responder};
use actix_files as fs;
use logger::Logger;
use mongodb::{Client, options::ClientOptions};
use serde_json::json;
use sockets::sockets::Lobby;

// try to convert all requests to reduce code
// async fn parse_and_run(request: String, function: &dyn Fn(Value) -> HttpResponse) -> HttpResponse {
//     let response: HttpResponse = match serde_json::from_str(&request) {
//         Ok(data) => function(data),
//         Err(_) => HttpResponse::NotAcceptable().body(json!({
//             "error": "Failed to parse request. Make sure it is a valid JSON payload."
//         }).to_string())
//     };
//     response
// }

#[get("/")]
async fn hello() -> impl Responder {
    fs::NamedFile::open_async("./src/landing/index.html").await
}

#[get("/landing/banner")]
async fn logo() -> impl Responder {
    fs::NamedFile::open_async("./src/landing/img/banner.png").await
}

#[get("/landing/icons/title")]
async fn icon() -> impl Responder {
    fs::NamedFile::open_async("./src/landing/img/logo.ico").await
}

#[get("/landing/icons/social")]
async fn social() -> impl Responder {
    fs::NamedFile::open_async("./src/landing/img/logo.png").await
}

#[get("/join/vehicle/{uid}")]
async fn joinvehicle(req: HttpRequest, stream: web::Payload, context: web::Data<Manager>, path: Path<String>) -> impl Responder {
    context.joinvehicle(path.into_inner(), &req, stream).await
}

#[get("/join/user/{vid}/{uid}")]
async fn joinuser(req: HttpRequest, stream: web::Payload, context: web::Data<Manager>, path: Path<(String, String)>) -> impl Responder {
    context.joinuser(path.0.clone(), path.1.clone(), &req, stream).await
}

#[get("/pair/{vid}/{uid}")]
async fn pair(req: HttpRequest, stream: web::Payload, context: web::Data<Manager>, path: Path<(String, String)>, query_params: web::Query<HashMap<String, String>>) -> impl Responder {
    context.pair(path.1.clone(), path.0.clone(), &req, stream, query_params.get("initial").map(|s| s == "true").unwrap_or(false)).await
}

// Account management routes

#[post("/login")]
async fn login(context: web::Data<Manager>, req_body: String) -> impl Responder {
    match serde_json::from_str(&req_body) {
        Ok(data) => context.login(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    }
}

#[post("/status")]
async fn status(context: web::Data<Manager>, req_body: String) -> impl Responder {
    match serde_json::from_str(&req_body) {
            Ok(data) => context.status(data),
            Err(_) => HttpResponse::NotAcceptable().body(json!({
                "error": "Failed to parse request. Make sure it is a valid JSON payload."
            }).to_string())
        }
}

#[post("/signup")]
async fn signup(context: web::Data<Manager>, req_body: String) -> impl Responder {
     match serde_json::from_str(&req_body) {
        Ok(data) => context.signup(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    }
}

#[post("/vehicle/register")]
async fn registervehicle(context: web::Data<Manager>, req_body:String) -> impl Responder {
    match serde_json::from_str(&req_body) {
        Ok(data) => context.registervehicle(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    }
}

#[post("/vehicle/refresh")]
async fn refreshvehicle(context: web::Data<Manager>, req_body: String) -> impl Responder {
    match serde_json::from_str(&req_body) {
        Ok(data) => context.refreshvehicles(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    }
}

#[post("/vehicle/edit")]
async fn editvehicle(context: web::Data<Manager>, req_body: String) -> impl Responder {
    context.editvehicle(req_body).await
}

// data management routes

#[post("/logs/daily")]
async fn dailylogs(context: web::Data<Manager>, req_body: String) -> impl Responder {
    context.dailylogs(req_body).await
}

#[post("/logs/periodic")]
async fn periodiclogs(context: web::Data<Manager>, req_body: String) -> impl Responder {
    context.periodiclogs(req_body).await
}

#[post("/logs/overall")]
async fn overall_logs(context: web::Data<Manager>, req_body: String) -> impl Responder {
    context.overall_logs(req_body).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:8080/").await.unwrap();
    client_options.app_name = Some("alpadrive".to_string());
    let client = Client::with_options(client_options).unwrap();
    let database = client.database("alpadrive");
    let active_vehicles = Arc::new(RwLock::new(HashMap::<String, String>::new()));
    let active_sessions = Arc::new(RwLock::new(0));
    let av_copy = Arc::clone(&active_vehicles);
    let sessions_copy = Arc::clone(&active_sessions);

    let lobby = Lobby::new(active_vehicles, active_sessions).await;
    let logger = Logger::new().await;
    // let lobby = Lobby::default().start();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Manager::start(database.clone(), lobby.clone(), logger.clone(), Arc::clone(&av_copy), Arc::clone(&sessions_copy))))
            .service(hello)
            .service(logo)
            .service(icon)
            .service(social)
            .service(login)
            .service(status)
            .service(signup)
            .service(registervehicle)
            .service(refreshvehicle)
            .service(editvehicle)
            .service(joinvehicle)
            .service(joinuser)
            .service(pair)
            .service(dailylogs)
            .service(periodiclogs)
            .service(overall_logs)
    })
    .bind(("127.0.0.1", 7878))?
    .run()
    .await
}