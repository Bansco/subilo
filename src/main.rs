use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
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
    #[serde(rename(serialize = "ref", deserialize = "ref"))]
    ref_: String,
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

fn run_project(project: &Project) {
    println!("Starting to run project {} commands", {
        &project.repository
    });

    for command in &project.commands {
        println!(
            "Running command \"{}\" at \"{}\" path",
            &command, &project.path
        );

        let path = shellexpand::tilde(&project.path).into_owned();
        let output = run_command(&path, &command);

        // TODO: Stream command stdout to log file instead of parse and log the
        //       whole response
        match str::from_utf8(&output.stdout) {
            Ok(stdout) => println!("{}", stdout),
            Err(error) => println!("There was a problem parsing stdout: {:?}", error),
        }

        if !output.status.success() {
            match output.status.code() {
                Some(code) => println!("Exit {}", code),
                None => println!("Process terminated by signal"),
            }
        }
    }
}

#[post("/webhook")]
async fn webhook(info: web::Json<PushEvent>) -> impl Responder {
    // TODO: 400? 500? on missing Threshfile ?
    // TODO: where should Threshfile be by default ?
    // TODO: accept flag for Threshfile location

    thread::spawn(move || {
        let threshfile =
            fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
        let config: Config = toml::from_str(&threshfile).expect("Failed parsing threshfile file");

        let maybe_project = config.projects.iter().find(|x| {
            x.username == info.repository.owner.login && x.repository == info.repository.name
        });

        match maybe_project {
            Some(project) => run_project(&project),
            None => println!("Project not found"),
        }
    });

    HttpResponse::Ok()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let contents = fs::read_to_string("./.threshfile").expect("Failed reading threshfile file");
    let config: Config = toml::from_str(&contents).expect("Failed parsing threshfile file");

    let localhost = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(localhost, config.port);

    HttpServer::new(|| App::new().service(webhook))
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
