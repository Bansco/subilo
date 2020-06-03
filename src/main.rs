use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct Project {
    username: String,
    repository: String,
    path: String,
    commands: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Config {
    projects: Vec<Project>,
}

#[get("/")]
async fn index() -> impl Responder {
    let contents =
        fs::read_to_string("./config.toml").expect("Something went wrong reading the file");

    let config: Config = toml::from_str(&contents).unwrap();

    println!("{:#?}", config);

    format!("{}", contents)
}

#[post("/webhook")]
async fn webhook() -> impl Responder {
    HttpResponse::Ok()
}

pub async fn start_server(address: &str) -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index).service(webhook))
        .bind(address)?
        .run()
        .await
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    start_server("127.0.0.1:9000").await
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn test_webhook_ok() {
        let mut server = test::init_service(App::new().service(index).service(webhook)).await;

        let payload = &[
            ("name", "tigrin"),
            ("city", "amsterdam"),
        ];
        let req = test::TestRequest::post()
            .uri("/webhook")
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&mut server, req).await;

        assert!(resp.status().is_success());
    }
}
