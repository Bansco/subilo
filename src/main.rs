use actix_cors::Cors;
use actix_http::error::ResponseError;
use actix_web::middleware::Logger;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use actix_web_httpauth::middleware::HttpAuthentication;
use async_std::fs as async_fs;
use async_std::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::Command;
use std::{fs, process, str, thread};

#[macro_use]
extern crate log;

mod auth;
mod cli;
mod errors;

use errors::ThreshError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum MetadataStatus {
    Started,
    Succeeded,
    Failed,
}

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    name: String,
    status: MetadataStatus,
    started_at: String,
    ended_at: Option<String>,
}

impl Metadata {
    fn to_json_string(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: Option<u16>,
    logs_dir: Option<String>,
    secret: Option<String>,
}

struct Context {
    threshfile: String,
    logs_dir: String,
    secret: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobsConfig {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WebhookPayload {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Project {
    name: String,
    path: String,
    commands: Vec<String>,
}

impl Project {
    fn title(&self) -> String {
        format!("Project {} at {}\n", self.name, self.path)
    }
}

// TODO: handle failure by returning Result
fn run_command(path: &str, command: &str, log: &std::fs::File) -> std::process::Output {
    let stdout = log.try_clone().expect("Failed to clone log file (stdout)");
    let stderr = log.try_clone().expect("Failed to clone log file (stderr)");

    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(stdout)
        .stderr(stderr)
        .current_dir(path)
        .spawn()
        .expect("Failed to execute child process")
        .wait_with_output()
        .expect("Failed to wait on child process")
}

fn create_job_name(repository: &str) -> String {
    let repository = repository.replace("/", "-");
    let now = Utc::now().format("%Y-%m-%d--%H-%M-%S").to_string();
    format!("{}_{}", repository, now)
}

fn create_log_name(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.log", log_dir, job)
}

fn create_metadata_log_name(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.json", log_dir, job)
}

fn run_project(
    project: Project,
    mut metadata: Metadata,
    mut log: std::fs::File,
    mut metadata_log: std::fs::File,
) {
    log.write_all(project.title().as_bytes()).unwrap();

    for command in &project.commands {
        debug!("Running command {}", &command);
        log.write_all(format!("$ {}\n", &command).as_bytes())
            .unwrap();

        let path = shellexpand::tilde(&project.path).into_owned();
        let output = run_command(&path, &command, &log);

        match (output.status.success(), output.status.code()) {
            (true, _) => (),
            (_, Some(code)) => {
                log.write_all(format!("Exit {}\n", code).as_bytes())
                    .unwrap();

                metadata.status = MetadataStatus::Failed;
                break;
            }
            (_, None) => {
                log.write_all("Process terminated by signal\n".to_string().as_bytes())
                    .unwrap();

                metadata.status = MetadataStatus::Failed;
                break;
            }
        }
    }

    if let MetadataStatus::Started = metadata.status {
        metadata.status = MetadataStatus::Succeeded;
    }
    metadata.ended_at = Some(Utc::now().to_rfc3339());
    metadata_log.seek(SeekFrom::Start(0)).unwrap();
    metadata_log
        .write_all(metadata.to_json_string().unwrap().as_bytes())
        .unwrap();
}

fn spawn_job(logs_dir: &str, project: Project) -> Result<String, ThreshError> {
    let job_name = create_job_name(&project.name);
    let file_name = create_log_name(&job_name, logs_dir);
    let metadata_file_name = create_metadata_log_name(&job_name, logs_dir);

    fs::create_dir_all(logs_dir).map_err(|err| ThreshError::CreateLogDir { source: err })?;

    let metadata = Metadata {
        name: project.name.clone(),
        status: MetadataStatus::Started,
        started_at: Utc::now().to_rfc3339(),
        ended_at: None,
    };

    let log =
        fs::File::create(file_name).map_err(|err| ThreshError::CreateLogFile { source: err })?;

    let mut metadata_log = OpenOptions::new()
        .write(true)
        .create(true)
        .open(metadata_file_name)
        .map_err(|err| ThreshError::CreateLogFile { source: err })?;

    metadata_log
        .write_all(metadata.to_json_string().unwrap().as_bytes())
        .map_err(|err| ThreshError::WriteLogFile { source: err })?;

    thread::spawn(move || {
        debug!("Starting to process {} project", &project.name);
        run_project(project, metadata, log, metadata_log)
    });

    Ok(job_name)
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
    match spawn_job(&ctx.logs_dir, project.unwrap()) {
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

        let name = path.file_name().unwrap().to_owned().into_string().unwrap();

        if name.ends_with(".log") {
            logs.push(name.replace(".log", ""));
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

    let metadata_json: Metadata = serde_json::from_str(&metadata)?;
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
