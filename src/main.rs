use actix_web::middleware::Logger;
use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use env_logger;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Command, Output, Stdio};
use std::str;
use std::thread;

#[derive(Debug, Deserialize, Serialize)]
struct GitHubUser {
    login: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repository {
    name: String,
    owner: GitHubUser,
}

// https://developer.github.com/webhooks/event-payloads/#push
#[derive(Debug, Deserialize, Serialize)]
struct PushEvent {
    zen: Option<String>,
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    ref_: Option<String>,
    repository: Repository,
    sender: GitHubUser,
}

#[derive(Debug, Deserialize, Serialize)]
struct Project {
    // TODO: add branch (pattern?) to filter on
    // TODO: add option to populate env
    username: String,
    repository: String,
    path: String,
    commands: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    port: u16,
    run_job_on_ping: Option<bool>,
    projects: Vec<Project>,
}

// TODO: handle failure by returning Result
fn run_command(path: &String, command: &String) -> Output {
    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .current_dir(path)
        .env_clear()
        .spawn()
        .expect("failed to execute child")
        .wait_with_output()
        .expect("failed to wait on child")
}

fn run_project(project: &Project) -> String {
    let mut result: String = "".to_owned();

    for command in &project.commands {
        result.push_str(&format!("$ {}\n", &command));

        let path = shellexpand::tilde(&project.path).into_owned();
        let output = run_command(&path, &command);

        // TODO: Stream command stdout to log file instead of parse and log the
        //       whole response
        match str::from_utf8(&output.stdout) {
            Ok(stdout) => result.push_str(&stdout),
            Err(error) => result.push_str(&format!("Failed to parse stdout: {:?}\n", error)),
        }

        match (output.status.success(), output.status.code()) {
            (true, _) => (),
            (_, Some(code)) => result.push_str(&format!("Exit {}\n", code)),
            (_, None) => result.push_str(&format!("Process terminated by signal\n")),
        }
    }

    result
}

// TODO: 400? 500? on missing Threshfile ?
// TODO: where should Threshfile be by default ?
// TODO: accept flag for Threshfile location
// TODO: respond right away and run 'job' asynchronously after
// TODO: write logs to file instead
#[post("/webhook")]
async fn webhook(info: web::Json<PushEvent>) -> impl Responder {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let is_ping = info.zen.is_some();
    let run_job_on_ping = config.run_job_on_ping.map_or(false, |x| x);

    if is_ping && !run_job_on_ping {
        return HttpResponse::Ok().body("200 Ok");
    }

    thread::spawn(move || {
        info.ref_
            .as_ref()
            .filter(|ref_| ref_ == &"refs/heads/master")
            .and_then(|_| {
                config.projects.iter().find(|x| {
                    x.username == info.repository.owner.login
                        && x.repository == info.repository.name
                })
            })
            .map(|project| {
                let output = run_project(&project);

                let logfile = format!(
                    "./log/{}_{}_{}.log",
                    Utc::now().format("%Y_%m_%d_%H_%M_%S").to_string(),
                    info.repository.owner.login,
                    info.repository.name,
                );

                fs::create_dir_all("./log").and_then(|_| fs::write(logfile, output.as_bytes()))
            });
    });

    HttpResponse::Ok().body("200 Ok")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, config.port);

    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| App::new().wrap(Logger::default()).service(webhook))
        .bind(socket)?
        .run()
        .await
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn test_webhook_ok() {
        let mut server = test::init_service(App::new().service(webhook)).await;

        let payload = &[("name", "tigrin"), ("city", "amsterdam")];
        let req = test::TestRequest::post()
            .uri("/webhook")
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&mut server, req).await;

        assert!(resp.status().is_success());
    }
}
