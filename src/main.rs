use actix_cors::Cors;
use actix_web::error::ResponseError;
use actix_web::middleware;
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

use crate::errors::SubiloError;

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

#[derive(Debug, Deserialize, Serialize)]
struct Context {
    subilofile: String,
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
    user: auth::User,
) -> Result<impl Responder> {
    if !user.has_permission("job:create".to_owned()) {
        warn!("User does not have permission to create a job");
        return Ok(HttpResponse::Forbidden().body("Forbidden"));
    }

    let subilo_file = async_fs::read_to_string(&ctx.subilofile)
        .await
        .map_err(|err| SubiloError::ReadSubiloFile { source: err })?;

    let jobs_config: JobsConfig =
        toml::from_str(&subilo_file).map_err(|err| SubiloError::ParseSubiloFile { source: err })?;

    debug!("Finding project by name {}", &body.name);
    let project = jobs_config
        .projects
        .into_iter()
        .find(|project| project.name == body.name);

    if project.is_none() {
        return Ok(HttpResponse::NotFound().body("Not Found"));
    }

    debug!("Creating job for project {}", &body.name);
    match core::spawn_job(&ctx.logs_dir, project.unwrap()) {
        // TODO: Migrate to JSON response.
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

        let file_name = match path.file_name() {
            Some(name) => name
                .to_owned()
                .into_string()
                .map_err(|_err| SubiloError::ReadFileName {})?,
            None => {
                error!("Failed to read file at path {:?}", path);
                continue;
            }
        };

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
        "subilo=debug,actix_web=info"
    } else {
        "subilo=info,actix_web=info"
    };

    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();

    let subilofile = matches
        .value_of("config")
        .map(|path| shellexpand::tilde(&path).into_owned())
        // It is safe to unwrap because the value has a clap default.
        .unwrap();

    debug!("Parsing subilofile");
    let subilo_file = fs::read_to_string(&subilofile).expect("Failed to read subilofile");
    let config: Config = toml::from_str(&subilo_file).expect("Failed to parse subilofile");
    // Parse only to validate the projects
    let _: JobsConfig = toml::from_str(&subilo_file).expect("Failed to parse subilofile");

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

        let duration: i64 = matches
            .value_of("duration")
            .and_then(|port| port.parse().ok())
            // It is safe to unwrap because the value has a clap default.
            .unwrap();

        let permissions = matches
            .value_of("permissions")
            .map(|permissions: &str| {
                permissions
                    .to_owned()
                    .split(',')
                    .map(|s| s.to_string())
                    .collect()
            })
            .map(|permissions: Vec<String>| {
                permissions
                    .into_iter()
                    .map(|permission| permission.trim().to_owned())
                    .collect()
            })
            // It is safe to unwrap because the value has a clap default.
            .unwrap();

        match auth::create_token(&secret, permissions, duration) {
            Ok(token) => println!("Bearer {}", token),
            Err(err) => eprintln!("Failed to create authentication token {}", err),
        }
        return Ok(());
    }

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, port);
    let context = web::Data::new(Context {
        subilofile,
        logs_dir,
        secret,
    });

    debug!("Creating logs directory at '{}'", &context.logs_dir);
    fs::create_dir_all(&context.logs_dir).expect("Failed to create logs directory");

    debug!("Attempting to bind Subilo agent to {}", &socket);
    let server_bound = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
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
            info!("Subilo agent bound to {}", &socket);
            server.run().await
        }
        Err(err) => {
            error!("Failed to bind Subilo agent to {}. Error: {}", &socket, err);
            Err(err)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use serde_json::Value;

    #[actix_rt::test]
    async fn test_webhook() {
        let context = web::Data::new(Context {
            subilofile: "./.subilofile".to_owned(),
            logs_dir: String::from("./logs"),
            secret: String::from("secret"),
        });

        let mut server = test::init_service(
            App::new()
                .wrap(middleware::Compress::default())
                .wrap(middleware::Logger::default())
                .app_data(context.clone())
                .wrap(HttpAuthentication::bearer(auth::validator))
                .service(webhook),
        )
        .await;

        let payload = r#"{ "name": "test" }"#;
        let json: Value = serde_json::from_str(payload).unwrap();

        let req = test::TestRequest::post()
            .uri("/webhook")
            .header("Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJleHAiOjE2MDk2MDU2NDIsImlhdCI6MTU5MzgzNzY0MiwiaXNzIjoic3ViaWxvOmFnZW50IiwidXNlciI6eyJwZXJtaXNzaW9ucyI6WyJqb2I6Y3JlYXRlIiwiam9iOnJlYWQiXX19.xKDuTsbug9XT5IXjnz_TYk-cIsCoqV11skXPa8XK054KFiouxh4jyOL7MX6wXwT1HMs2Mn-r6Ygvuhj-M71Bxg")
            .set_json(&json)
            .to_request();

        let res = test::call_service(&mut server, req).await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
