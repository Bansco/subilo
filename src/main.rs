use actix_cors::Cors;
use actix_web::error::ResponseError;
use actix_web::middleware::Logger;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use actix_web_httpauth::middleware::HttpAuthentication;
use async_std::fs as async_fs;
use async_std::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{fs, process, str};

#[macro_use]
extern crate log;

mod auth;
mod cli;
mod core;
mod errors;

use crate::errors::ThreshError;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: Option<u16>,
    logs_dir: Option<String>,
    secret: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobsConfig {
    projects: Vec<core::Project>,
}

struct Context {
    threshfile: String,
    logs_dir: String,
    secret: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WebhookPayload {
    name: String,
}

#[post("/healthz")]
async fn healthz() -> impl Responder {
    HttpResponse::Ok().body("200 Ok")
}

#[post("/info")]
async fn info() -> Result<web::Json<serde_json::value::Value>> {
    let response = json!({ "version": env!("CARGO_PKG_VERSION") });
    Ok(web::Json(response))
}

#[post("/webhook")]
async fn webhook(
    body: web::Json<WebhookPayload>,
    ctx: web::Data<Context>,
) -> Result<impl Responder> {
    debug!("Parsing threshfile");
    let thresh_file = async_fs::read_to_string(&ctx.threshfile)
        .await
        .map_err(|err| ThreshError::ReadThreshFile { source: err })?;

    let jobs_config: JobsConfig =
        toml::from_str(&thresh_file).map_err(|err| ThreshError::ParseThreshFile { source: err })?;

    debug!("Finding project by name {}", &body.name);
    let project = jobs_config
        .projects
        .into_iter()
        .find(|project| project.name == body.name);

    if project.is_none() {
        return Ok(HttpResponse::NotFound().body("404 Not Found"));
    }

    debug!("Creating job for project {}", &body.name);
    match core::spawn_job(&ctx.logs_dir, project.unwrap()) {
        Ok(job_id) => Ok(HttpResponse::Ok().body(format!("200 Ok\nJob: {}", job_id))),
        Err(err) => Ok(err.error_response()),
    }
}

#[get("/jobs")]
async fn get_jobs(ctx: web::Data<Context>) -> Result<web::Json<serde_json::value::Value>> {
    let log_dir = shellexpand::tilde(&ctx.logs_dir).into_owned();
    let mut logs: Vec<String> = Vec::new();
    let mut dir = async_std::fs::read_dir(log_dir).await?;

    while let Some(entry) = dir.next().await {
        let path = entry?.path();

        if path.file_name().is_none() {
            error!("Failed to read file at path {:?}", path);
            continue;
        }

        let file_name = path
            .file_name()
            .unwrap()
            .to_owned()
            .into_string()
            .map_err(|_err| ThreshError::ReadFileName {})?;

        if file_name.ends_with(".json") {
            logs.push(file_name.replace(".json", ""));
        }
    }

    Ok(web::Json(serde_json::to_value(&logs)?))
}

#[get("/jobs/{job_name}")]
async fn get_job_by_name(
    job_name: web::Path<String>,
    ctx: web::Data<Context>,
) -> Result<web::Json<serde_json::value::Value>> {
    let log_dir = shellexpand::tilde(&ctx.logs_dir).into_owned();

    let log_file_name = format!("{}/{}.log", &log_dir, job_name);
    let metadata_file_name = format!("{}/{}.json", &log_dir, job_name);

    let log = async_std::fs::read_to_string(log_file_name).await?;
    let metadata = async_std::fs::read_to_string(metadata_file_name).await?;

    let metadata_json: core::Metadata = serde_json::from_str(&metadata)?;
    let response = json!({ "log": log, "metadata": metadata_json });

    Ok(web::Json(response))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let matches = cli::ask().get_matches();

    let log_level = if matches.is_present("verbose") {
        "thresh=debug,actix_web=info"
    } else {
        "thresh=info,actix_web=info"
    };

    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();

    let threshfile = matches
        .value_of("config")
        .map(|path| shellexpand::tilde(&path).into_owned())
        .unwrap_or_else(|| "./.threshfile".to_owned());

    debug!("Parsing threshfile");
    let thresh_file = fs::read_to_string(&threshfile).expect("Failed to read threshfile");
    let config: Config = toml::from_str(&thresh_file).expect("Failed to parse threshfile");
    // Parse only to validate the projects
    let _: JobsConfig = toml::from_str(&thresh_file).expect("Failed to parse threshfile");

    let default_port = 8080;
    let default_logs_dir = "./logs".to_owned();

    let port: u16 = matches
        .value_of("port")
        .and_then(|port| port.parse().ok())
        .or(config.port)
        .unwrap_or(default_port);

    let logs_dir = matches
        .value_of("logs-dir")
        .map(|s| s.to_string())
        .or(config.logs_dir)
        .unwrap_or(default_logs_dir);

    let maybe_secret = matches
        .value_of("secret")
        .map(|s| s.to_string())
        .or(config.secret);

    let secret = match maybe_secret {
        Some(secret) => secret,
        None => {
            debug!("Required \"secret\" argument was not provided. Exiting process with status 1");
            eprintln!("Secret is required");
            process::exit(1);
        }
    };

    if matches.subcommand_matches("token").is_some() {
        debug!("Creating authentication token");
        match auth::create_token(&secret) {
            Ok(token) => println!("Bearer {}", token),
            Err(err) => eprintln!("Failed to create authentication token {}", err),
        }
        return Ok(());
    }

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, port);
    let context = web::Data::new(Context {
        threshfile,
        logs_dir,
        secret,
    });

    debug!("Creating logs directory at '{}'", &context.logs_dir);
    fs::create_dir_all(&context.logs_dir).expect("Failed to create logs directory");

    debug!("Attempting to bind Thresh agent to {}", &socket);
    let server_bound = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(context.clone())
            .wrap(HttpAuthentication::bearer(auth::validator))
            .wrap(Cors::new().supports_credentials().finish())
            .service(healthz)
            .service(info)
            .service(webhook)
            .service(get_jobs)
            .service(get_job_by_name)
    })
    .bind(socket);

    match server_bound {
        Ok(server) => {
            info!("Thresh agent bound to {}", &socket);
            server.run().await
        }
        Err(err) => {
            error!("Failed to bind Thresh agent to {}. Error: {}", &socket, err);
            Err(err)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test;
    use serde_json::Value;

    #[actix_rt::test]
    async fn test_webhook() {
        let context = web::Data::new(Context {
            threshfile: "./.threshfile".to_owned(),
            logs_dir: String::from("./logs"),
            secret: String::from("secret"),
        });
        let mut server =
            test::init_service(App::new().app_data(context.clone()).service(webhook)).await;

        let payload = r#"{ "name": "test" }"#;
        let json: Value = serde_json::from_str(payload).unwrap();

        let req = test::TestRequest::post()
            .uri("/webhook")
            .set_json(&json)
            .to_request();
        let res = test::call_service(&mut server, req).await;

        assert!(res.status().is_success());
    }
}
