mod manager;

pub use crate::manager::Manager;
use actix::{Actor, StreamHandler};
use actix_web::{web, get, post, App, HttpResponse, HttpRequest, Error, HttpServer, Responder};
use actix_web_actors::ws;
use mongodb::{Client, options::ClientOptions};
use serde_json::json;

struct PairSocket {
    manager: web::Data<Manager>
}

impl Actor for PairSocket {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PairSocket {
    fn handle(&mut self, message: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match message {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                self.manager.echo();
                ctx.text(text)
            },
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => ()
        }
    }
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/vehicle/pair")]
async fn pairvehicle(context: web::Data<Manager>, req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(PairSocket {manager: context}, &req, stream);
    resp
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

#[post("/vehicle/register")]
async fn registervehicle(context: web::Data<Manager>, req_body:String) -> impl Responder {
    match serde_json::from_str(&req_body) {
        Ok(data) => context.registervehicle(data).await,
        Err(_) => HttpResponse::NotAcceptable().body(json!({
            "error": "Failed to parse request. Make sure it is a valid JSON payload."
        }).to_string())
    }
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
            .service(registervehicle)
            .service(pairvehicle)
    })
    .bind(("127.0.0.1", 7878))?
    .run()
    .await
}