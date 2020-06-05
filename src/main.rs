use actix_web::middleware::Logger;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Command, Output, Stdio};
use std::str;
use std::thread;
#[macro_use]
extern crate log;
use actix_files::NamedFile;
use std::io;
use std::io::Write;

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    full_name: String,
    name: String,
}

// https://developer.github.com/webhooks/event-payloads/#push
#[derive(Debug, Deserialize, Serialize)]
struct PushEvent {
    zen: Option<String>,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    ref_: Option<String>,
    repository: Repository,
}

#[derive(Debug, Deserialize, Serialize)]
struct Project {
    // TODO: add option to populate env
    repository: String,
    branch: String,
    path: String,
    commands: Vec<String>,
}

impl Project {
    fn title(&self) -> String {
        format!(
            "Project {} on branch {} at {}\n",
            self.repository, self.branch, self.path
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: u16,
    log: String,
    run_job_on_ping: Option<bool>,
    projects: Vec<Project>,
}

// TODO: handle failure by returning Result
fn run_command(path: &str, command: &str) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .current_dir(path)
        .spawn()
        .expect("failed to execute child")
        .wait_with_output()
        .expect("failed to wait on child")
}

fn job_name(repository: &str) -> String {
    let repository = repository.replace("/", "-");
    let now = Utc::now().format("%Y-%m-%d--%H-%M-%S").to_string();
    format!("{}_{}", repository, now)
}

fn job_logs(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.log", log_dir, job)
}

fn run_project(project: Project, mut log: std::fs::File) {
    log.write_all(project.title().as_bytes()).unwrap();

    for command in &project.commands {
        debug!("Running command {}", &command);
        log.write_all(format!("$ {}\n", &command).as_bytes())
            .unwrap();

        let path = shellexpand::tilde(&project.path).into_owned();
        let output = run_command(&path, &command);

        // TODO: Stream command stdout to log file instead of parse and log the
        //       whole response
        match str::from_utf8(&output.stdout) {
            Ok(stdout) => log.write_all(stdout.as_bytes()).unwrap(),
            Err(error) => log
                .write_all(format!("Failed to parse stdout: {:?}\n", error).as_bytes())
                .unwrap(),
        }

        match (output.status.success(), output.status.code()) {
            (true, _) => (),
            (_, Some(code)) => log
                .write_all(format!("Exit {}\n", code).as_bytes())
                .unwrap(),
            (_, None) => log
                .write_all("Process terminated by signal\n".to_string().as_bytes())
                .unwrap(),
        }
    }
}

// TODO: 400? 500? on missing Threshfile ?
// TODO: where should Threshfile be by default ?
// TODO: accept flag for Threshfile location
#[post("/webhook")]
async fn webhook(body: web::Json<PushEvent>) -> impl Responder {
    debug!("Github webhook recieved");

    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let is_ping = body.zen.is_some();
    let run_job_on_ping = &config.run_job_on_ping.map_or(false, |x| x);

    if is_ping && !run_job_on_ping {
        debug!("Retuning 200 status code to ping webhook");
        return HttpResponse::Ok().body("200 Ok");
    }

    let project = config
        .projects
        .into_iter()
        .find(|project| project.repository == body.repository.full_name)
        .filter(|project| match &body.ref_ {
            Some(ref_) => ref_.ends_with(&project.branch).to_owned(),
            None => false,
        });

    if project.is_none() {
        warn!(
            "Webhook not found for repository {}",
            body.repository.full_name
        );
        return HttpResponse::NotFound().body("404 Not Found");
    }

    let project = project.unwrap();

    let job_name = job_name(&body.repository.full_name);
    let file_name = job_logs(&job_name, &config.log);

    // Make sure logs directory exists
    let log = fs::create_dir_all(&config.log)
        .and_then(|_| fs::File::create(file_name))
        .expect("Failed creating log file");

    thread::spawn(move || {
        debug!("Starting to process {} project", &project.repository);
        run_project(project, log)
    });

    HttpResponse::Ok().body(format!("200 Ok\nJob: {}", job_name))
}

#[get("/logs")]
async fn get_logs() -> impl Responder {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let log_dir = shellexpand::tilde(&config.log).into_owned();
    let logs = fs::read_dir(log_dir)
        .unwrap()
        // TODO remove ".log"
        .map(|res| res.map(|e| e.path().file_name().unwrap().to_owned()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    HttpResponse::Ok().body(format!("{:?}", logs))
}

#[get("/logs/{log_name}")]
async fn get_log(log_name: web::Path<String>) -> Result<NamedFile> {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let log_dir = shellexpand::tilde(&config.log).into_owned();
    let path = format!("{}/{}.log", &log_dir, log_name);
    Ok(NamedFile::open(path)?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, config.port);

    std::env::set_var("RUST_LOG", "thresh=debug,actix_web=info");
    env_logger::init();

    fs::create_dir_all(config.log).expect("Failed creating logs directory");

    info!("Starting Thresh at {}", &socket);
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(webhook)
            .service(get_logs)
            .service(get_log)
    })
    .bind(socket)?
    .run()
    .await
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test;
    use serde_json::Value;

    #[actix_rt::test]
    async fn test_webhook_ok() {
        let mut server = test::init_service(App::new().service(webhook)).await;
        let payload = r#"
        {
            "ref": "refs/tags/master",
            "repository": {
                "name": "test",
                "full_name": "test/test"
            }
        }"#;
        let json: Value = serde_json::from_str(payload).unwrap();

        let req = test::TestRequest::post()
            .uri("/webhook")
            .set_json(&json)
            .to_request();

        let resp = test::call_service(&mut server, req).await;

        assert!(resp.status().is_success());
    }
}
