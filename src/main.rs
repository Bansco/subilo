use actix::prelude::*;
use actix_cors::Cors;
use actix_web::error::ResponseError;
use actix_web::middleware;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use actix_web_httpauth::middleware::HttpAuthentication;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{fs, process, str};

#[macro_use]
extern crate log;

mod auth;
mod cli;
mod core;
mod database;
mod errors;
mod job;

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
pub struct ProjectsInfo {
    projects: Vec<core::ProjectInfo>,
}

#[derive(Clone)]
pub struct Context {
    subilorc: String,
    logs_dir: String,
    secret: String,
    database: Addr<database::Database>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WebhookPayload {
    name: String,
}

#[get("/healthz")]
async fn healthz() -> impl Responder {
    HttpResponse::Ok().body("200 Ok")
}

#[get("/info")]
async fn info() -> Result<HttpResponse> {
    let response = json!({ "version": env!("CARGO_PKG_VERSION") });
    Ok(HttpResponse::Ok().json(response))
}

#[get("/projects")]
async fn list_projects(ctx: web::Data<Context>) -> Result<HttpResponse> {
    let subilorc_file = tokio::fs::read_to_string(&ctx.subilorc)
        .await
        .map_err(|err| SubiloError::ReadSubiloRC { source: err })?;

    let projects_info: ProjectsInfo =
        toml::from_str(&subilorc_file).map_err(|err| SubiloError::ParseSubiloRC { source: err })?;

    Ok(HttpResponse::Ok().json(projects_info))
}

#[derive(Debug, Deserialize, Serialize)]
struct WebhookResponse {
    name: String,
}

#[post("/webhook")]
async fn webhook(
    body: web::Json<WebhookPayload>,
    ctx: web::Data<Context>,
    user: auth::User,
) -> Result<impl Responder> {
    if !user.has_permission(auth::Permissions::JobWrite) {
        debug!("User does not have permission to create a job");
        return Ok(HttpResponse::Forbidden().body("Forbidden"));
    }

    let subilorc_file = tokio::fs::read_to_string(&ctx.subilorc)
        .await
        .map_err(|err| SubiloError::ReadSubiloRC { source: err })?;

    let jobs_config: JobsConfig =
        toml::from_str(&subilorc_file).map_err(|err| SubiloError::ParseSubiloRC { source: err })?;

    debug!("Finding project by name '{}'", &body.name);
    let project = jobs_config
        .projects
        .into_iter()
        .find(|project| project.name == body.name);

    if project.is_none() {
        return Ok(HttpResponse::NotFound().body("Not Found"));
    }

    let context = (*ctx.into_inner()).clone();
    match core::spawn_job(project.unwrap(), context).await {
        Ok(job_id) => Ok(HttpResponse::Ok().json(WebhookResponse { name: job_id })),
        Err(err) => Ok(err.error_response()),
    }
}

#[get("/jobs")]
async fn get_jobs(ctx: web::Data<Context>) -> Result<HttpResponse> {
    let query = database::Query {
        query: job::query::GET_ALL_JOBS.to_owned(),
        params: vec![],
        map_result: |row| {
            Ok(job::PartialJob {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                project: row.get(3)?,
                started_at: row.get(4)?,
                ended_at: row.get(5)?,
            })
        },
    };

    let jobs = ctx
        .database
        .send(query)
        .await
        .map_err(|err| SubiloError::DatabaseActor { source: err })?
        .map_err(|err| SubiloError::DatabaseQuery { source: err })?;

    let res = HttpResponse::Ok().json(jobs);
    Ok(res)
}

#[get("/jobs/{id}")]
async fn get_job_by_id(id: web::Path<String>, ctx: web::Data<Context>) -> Result<HttpResponse> {
    let query = database::Query {
        query: job::query::GET_JOB_BY_ID.to_owned(),
        params: vec![id.to_string()],
        map_result: |row| {
            let commands: serde_json::Value = row.get(4)?;
            Ok(job::Job {
                commands,
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                project: row.get(3)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
            })
        },
    };

    let jobs = ctx
        .database
        .send(query)
        .await
        .map_err(|err| SubiloError::DatabaseActor { source: err })?
        .map_err(|err| SubiloError::DatabaseQuery { source: err })?;

    let res = HttpResponse::Ok().json(jobs.first());
    Ok(res)
}

#[get("/jobs/{name}/log")]
async fn get_job_log_by_name(
    name: web::Path<String>,
    ctx: web::Data<Context>,
) -> Result<HttpResponse> {
    let log_dir = shellexpand::tilde(&ctx.logs_dir).into_owned();
    let log_file_name = format!("{}/{}.log", &log_dir, name);

    let log = tokio::fs::read_to_string(log_file_name).await?;
    let res = HttpResponse::Ok().body(log);

    Ok(res)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let subilo_path = shellexpand::tilde("~/.subilo");
    let matches = cli::ask(subilo_path.as_ref()).get_matches();

    let log_level = if matches.is_present("verbose") {
        "subilo=debug,actix_web=info"
    } else {
        "subilo=info,actix_web=info"
    };

    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();

    let maybe_secret = matches.value_of("secret").map(|s| s.to_string());

    let secret = match maybe_secret {
        Some(secret) => secret,
        None => {
            debug!("Required \"secret\" argument was not provided. Exiting process with status 1");
            eprintln!("Secret is required");
            process::exit(1);
        }
    };

    if let Some(token_matches) = matches.subcommand_matches("token") {
        debug!("Creating authentication token");

        let duration: i64 = token_matches
            .value_of("duration")
            .and_then(|duration| duration.parse().ok())
            .unwrap(); // Safe to unwrap, has clap default

        let permissions = token_matches
            .value_of("permissions")
            .map(|permissions: &str| {
                permissions
                    .to_owned()
                    .split(',')
                    .map(|s| serde_json::from_str(&format!("\"{}\"", s.to_string().trim())))
                    .filter_map(Result::ok)
                    .collect()
            })
            .unwrap(); // Safe to unwrap, has clap default

        match auth::create_token(&secret, permissions, duration) {
            Ok(token) => println!("Bearer {}", token),
            Err(err) => eprintln!("Failed to create authentication token {}", err),
        }

        return Ok(());
    }

    match matches.subcommand_matches("serve") {
        Some(serve_matches) => {
            let subilorc = serve_matches
                .value_of("config")
                .map(|path| shellexpand::tilde(&path).into_owned())
                .unwrap(); // Safe to unwrap, has clap default

            debug!("Parsing .subilorc file");
            // Parse only to validate the projects configuration
            let subilorc_file =
                fs::read_to_string(&subilorc).expect("Failed to read subilorc file");
            let _: JobsConfig =
                toml::from_str(&subilorc_file).expect("Failed to parse subilorc file");

            let port: u16 = serve_matches
                .value_of("port")
                .and_then(|port| port.parse().ok())
                .unwrap(); // Safe to unwrap, has clap default

            let logs_dir = serve_matches
                .value_of("logs-dir")
                .map(|s| s.to_string())
                .unwrap(); // Safe to unwrap, has clap default

            let database_path = serve_matches.value_of("database").unwrap(); // Safe to unwrap, has clap default

            debug!("Connecting to the local database");
            let db = database::Database::create(|_ctx| database::Database::new(database_path));

            let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
            let socket = SocketAddr::new(localhost, port);
            let context = web::Data::new(Context {
                subilorc,
                logs_dir,
                secret,
                database: db.clone(),
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
                    .service(list_projects)
                    .service(webhook)
                    .service(get_jobs)
                    .service(get_job_by_id)
                    .service(get_job_log_by_name)
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
        None => Ok(()),
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
        let db = database::Database::create(|_ctx| database::Database::new("test"));
        let context = web::Data::new(super::Context {
            subilorc: "./.subilorc".to_owned(),
            logs_dir: "./logs".to_owned(),
            secret: "secret".to_owned(),
            database: db,
        });

        let mut server = test::init_service(
            App::new()
                .wrap(middleware::Compress::default())
                .wrap(middleware::Logger::default())
                .app_data(context.clone())
                .wrap(HttpAuthentication::bearer(auth::validator))
                .wrap(Cors::new().supports_credentials().finish())
                .service(webhook),
        )
        .await;

        let payload = r#"{ "name": "test" }"#;
        let json: Value = serde_json::from_str(payload).unwrap();

        let req = test::TestRequest::post()
            .uri("/webhook")
            .header("Authorization", "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJleHAiOjE2MDM2MDA0MDcsImlhdCI6MTYwMDk3MjQwNywiaXNzIjoic3ViaWxvOmFnZW50IiwidXNlciI6eyJwZXJtaXNzaW9ucyI6WyJqb2I6d3JpdGUiXX19.mSQylmOBqXu87zX7mP-RMio2AbRGvZrj9v3Y4iN_rT4MdtEu4wwo2Ov4oblUv7rWHEh_lJTwROFSbVEnBNxHbg")
            .set_json(&json)
            .to_request();

        let res = test::call_service(&mut server, req).await;

        assert_eq!(res.status(), StatusCode::OK);
    }
}
