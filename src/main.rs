use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Command, Output, Stdio};
use std::str;

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
    // TODO: add port to run on
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
        let path = shellexpand::tilde(&project.path).into_owned();

        let output = run_command(&path, &command);

        result.push_str(&format!("$ {}\n", command));

        result.push_str(&format!("{}\n", str::from_utf8(&output.stdout).unwrap()));

        if !output.status.success() {
            result.push_str(&match output.status.code() {
                Some(code) => format!("Exit {}", code),
                None => format!("Process terminated by signal"),
            });
            break;
        }
    }

    result
}

#[post("/webhook")]
async fn webhook(info: web::Json<PushEvent>) -> impl Responder {
    // TODO: 400? 500? on missing Threshfile ?
    // TODO: where should Threshfile be by default ? 
    // TODO: accept flag for Threshfile location
    let contents =
        fs::read_to_string("./.threshfile").expect("Something went wrong reading the file");

    let config: Config = toml::from_str(&contents).unwrap();

    let maybe_project = config.projects.iter().find(|x| {
        x.username == info.repository.owner.login && x.repository == info.repository.name
    });

    match maybe_project {
        // TODO: respond right away and run 'job' asynchronously after
        // TODO: write logs to file instead
        Some(project) => HttpResponse::Ok().body(run_project(&project)),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(webhook))
        .bind("127.0.0.1:9000")?
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
