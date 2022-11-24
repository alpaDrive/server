mod manager;

pub use crate::manager::Manager;
use actix::Actor;
use actix_web::{web::{self, Path}, get, post, App, HttpResponse, HttpRequest, HttpServer, Responder};
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
    HttpResponse::Ok().body("alpaDrive API")
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
async fn pair(req: HttpRequest, stream: web::Payload, context: web::Data<Manager>, path: Path<(String, String)>) -> impl Responder {
    context.pair(path.1.clone(), path.0.clone(), &req, stream).await
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017/").await.unwrap();
    client_options.app_name = Some("alpadrive".to_string());
    let client = Client::with_options(client_options).unwrap();
    let database = client.database("alpadrive");

    let lobby = Lobby::default().start();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Manager::start(database.clone(), lobby.clone())))
            .service(hello)
            .service(login)
            .service(status)
            .service(signup)
            .service(registervehicle)
            .service(refreshvehicle)
            .service(joinvehicle)
            .service(joinuser)
            .service(pair)
    })
    .bind(("127.0.0.1", 7878))?
    .run()
    .await
}