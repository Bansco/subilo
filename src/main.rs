use hex::FromHex;
use actix_web::middleware::Logger;
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Command, Output};
use std::str;
use std::thread;
#[macro_use]
extern crate log;
use actix_files::NamedFile;
use actix_web::web::Bytes;
// use crypto::hmac::Hmac;
// use crypto::mac::Mac;
// use crypto::sha1::Sha1;
use std::io;
use std::io::Write;

mod auth;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: Option<u16>,
    logs_dir: Option<String>,
    run_job_on_ping: Option<bool>,
}

struct Context {
    threshfile: String,
    logs_dir: String,
    run_job_on_ping: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobsConfig {
    projects: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    full_name: String,
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

// TODO: handle failure by returning Result
fn run_command(path: &str, command: &str, log: &std::fs::File) -> Output {
    let stdout = log.try_clone().expect("Failed to clone log file (stdout)");
    let stderr = log.try_clone().expect("Failed to clone log file (stderr)");

    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(stdout)
        .stderr(stderr)
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
        let output = run_command(&path, &command, &log);

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
async fn webhook(req: HttpRequest, raw_body: Bytes, ctx: web::Data<Context>) -> impl Responder {
    debug!("Github webhook recieved");

    // Start auth
    let secret = b"secret";
    let hash_signature = req.headers().get("X-Hub-Signature");
    if hash_signature.is_none() {
        return HttpResponse::Forbidden().body("X-Hub-Signature not found in the request");
    }
    let hash_signature = hash_signature.unwrap().to_str().ok().unwrap();
    let hash_signature = hex::decode(hash_signature).expect("sarasa");

    if !auth::validate(secret, &hash_signature, &raw_body) {
        return HttpResponse::Forbidden().body("X-Hub-Signature not found in the request");
    }
    // End auth

    let body: PushEvent = serde_json::from_slice(raw_body.as_ref()).unwrap();
    let thresh_file = fs::read_to_string(&ctx.threshfile).expect("Failed reading threshfile file");
    let jobs_config: JobsConfig =
        toml::from_str(&thresh_file).expect("Failed parsing threshfile file");

    let is_ping = body.zen.is_some();
    if is_ping && !&ctx.run_job_on_ping {
        debug!("Retuning 200 status code to ping webhook");
        return HttpResponse::Ok().body("200 Ok");
    }

    let job_name = jobs_config
        .projects
        .into_iter()
        .find(|project| project.repository == body.repository.full_name)
        .filter(|project| match &body.ref_ {
            Some(ref_) => ref_.ends_with(&project.branch).to_owned(),
            None => false,
        })
        .map(|project| {
            let job_name = job_name(&body.repository.full_name);
            let file_name = job_logs(&job_name, &ctx.logs_dir);

            // Make sure logs directory exists
            let log = fs::create_dir_all(&ctx.logs_dir)
                .and_then(|_| fs::File::create(file_name))
                .expect("Failed creating log file");

            thread::spawn(move || {
                debug!("Starting to process {} project", &project.repository);
                run_project(project, log)
            });

            job_name
        });

    match job_name {
        Some(job_name) => HttpResponse::Ok().body(format!("200 Ok\nJob: {}", job_name)),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[get("/logs")]
async fn get_logs(ctx: web::Data<Context>) -> impl Responder {
    let log_dir = shellexpand::tilde(&ctx.logs_dir).into_owned();
    let logs = fs::read_dir(log_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path().file_name().unwrap().to_owned()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    HttpResponse::Ok().body(format!("{:?}", logs))
}

#[get("/logs/{log_name}")]
async fn get_log(log_name: web::Path<String>, ctx: web::Data<Context>) -> Result<NamedFile> {
    let log_dir = shellexpand::tilde(&ctx.logs_dir).into_owned();
    let path = format!("{}/{}.log", &log_dir, log_name);

    Ok(NamedFile::open(path)?)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Path to the Threshfile")
                .takes_value(true)
                .default_value("./.threshfile"),
        )
        .arg(
            clap::Arg::with_name("port")
                .short("p")
                .long("port")
                .help("Sets a custom server port")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("logs-dir")
                .short("l")
                .long("logs-dir")
                .help("Sets a custom logs directory")
                .takes_value(true),
        )
        .get_matches();

    let threshfile = matches
        .value_of("config")
        .map(|path| shellexpand::tilde(&path).into_owned())
        .unwrap_or_else(|| "./.threshfile".to_owned());

    let thresh_file = fs::read_to_string(&threshfile).expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&thresh_file).expect("Failed parsing threshfile file");

    let run_job_on_ping = config.run_job_on_ping;
    let port: u16 = matches
        .value_of("port")
        .map(|port| port.parse().unwrap())
        .unwrap_or_else(|| config.port.unwrap_or(8080));

    let logs_dir = matches
        .value_of("logs-dir")
        .unwrap_or(&config.logs_dir.unwrap_or_else(|| "./logs".to_owned()))
        .to_owned();

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, port);
    let context = web::Data::new(Context {
        threshfile,
        logs_dir,
        run_job_on_ping: run_job_on_ping.unwrap_or(false),
    });

    std::env::set_var("RUST_LOG", "thresh=debug,actix_web=info");
    env_logger::init();

    fs::create_dir_all(&context.logs_dir).expect("Failed creating logs directory");

    info!("Starting Thresh at {}", &socket);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(context.clone())
            .service(webhook)
            .service(get_log)
            .service(get_logs)
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
    async fn test_webhook() {
        let context = web::Data::new(Context {
            threshfile: "./.threshfile".to_owned(),
            logs_dir: String::from("./logs"),
            run_job_on_ping: false,
        });
        let mut server =
            test::init_service(App::new().app_data(context.clone()).service(webhook)).await;

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
        let res = test::call_service(&mut server, req).await;

        assert!(res.status().is_success());
    }

    #[actix_rt::test]
    async fn test_webhook_ping() {
        let context = web::Data::new(Context {
            threshfile: "./.threshfile".to_owned(),
            logs_dir: String::from("./logs"),
            run_job_on_ping: false,
        });
        let mut server =
            test::init_service(App::new().app_data(context.clone()).service(webhook)).await;

        let payload = r#"
        {
            "zen": "no es moco de pavo",
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
        let res = test::call_service(&mut server, req).await;

        assert!(res.status().is_success());
    }
}
