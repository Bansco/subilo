use std::fs;
use actix_web::{get, App, HttpServer, Responder};
use serde::Deserialize;

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
    let contents = fs::read_to_string("./config.toml")
        .expect("Something went wrong reading the file");

    let config: Config = toml::from_str(&contents).unwrap();

    println!("{:#?}", config);

    format!("{}", contents)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:9000")?
        .run()
        .await
}
