use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::process::Command;
use std::{fs, str, thread};

use crate::errors::ThreshError;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetadataStatus {
    Started,
    Succeeded,
    Failed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    pub name: String,
    pub status: MetadataStatus,
    pub started_at: String,
    pub ended_at: Option<String>,
}

impl Metadata {
    fn to_json_string(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub commands: Vec<String>,
}

impl Project {
    fn title(&self) -> String {
        format!("Project {} at {}\n", self.name, self.path)
    }
}

// TODO: handle failure by returning Result
pub fn run_command(path: &str, command: &str, log: &std::fs::File) -> std::process::Output {
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

pub fn create_job_name(repository: &str) -> String {
    let repository = repository.replace("/", "-");
    let now = Utc::now().format("%Y-%m-%d--%H-%M-%S").to_string();
    format!("{}_{}", repository, now)
}

pub fn create_log_name(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.log", log_dir, job)
}

pub fn create_metadata_log_name(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.json", log_dir, job)
}

pub fn run_project(
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

pub fn spawn_job(logs_dir: &str, project: Project) -> Result<String, ThreshError> {
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