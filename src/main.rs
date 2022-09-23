mod manager;

pub use crate::manager::Manager;
use actix_web::{web, get, post, App, HttpResponse, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/login")]
async fn login(context: web::Data<Manager>, req_body: String) -> impl Responder {
    &context.login("test".to_string(), "test".to_string());
    HttpResponse::Ok().body(req_body)
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(Manager::start()))
            .service(hello)
            .service(echo)
            .service(login)
    })
    .bind(("127.0.0.1", 7878))?
    .run()
    .await
}