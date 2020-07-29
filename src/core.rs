use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{Seek, SeekFrom, Write};
use std::process::Command;
use std::{str, thread};

use crate::errors::SubiloError;
use crate::job;

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
    pub fn to_json_string(&self) -> Result<String, SubiloError> {
        serde_json::to_string(&self)
            .map_err(|err| SubiloError::SerializeMetadataToJSON { source: err })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub commands: Vec<String>,
}

impl Project {
    fn description(&self) -> String {
        format!("Project '{}' at {}\n", self.name, self.path)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub home: Option<String>,
    pub ci: Option<String>,
    pub repo: Option<String>,
    pub commands: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum RunError {
    #[error("[FATAL] Failed to clone log file, {}", source)]
    CloneLogFile { source: std::io::Error },

    #[error("[FATAL] Failed to execute as child process: {}", source)]
    ExecuteCommand { source: std::io::Error },
}

pub fn run_command(
    path: &str,
    command: &str,
    log: &std::fs::File,
) -> Result<std::process::Output, RunError> {
    let stdout = log
        .try_clone()
        .map_err(|err| RunError::CloneLogFile { source: err })?;
    let stderr = log
        .try_clone()
        .map_err(|err| RunError::CloneLogFile { source: err })?;

    Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(stdout)
        .stderr(stderr)
        .current_dir(path)
        .spawn()
        .map_err(|err| RunError::ExecuteCommand { source: err })?
        .wait_with_output()
        .map_err(|err| RunError::ExecuteCommand { source: err })
}

pub fn run_project_deployment(
    project: Project,
    witness: job::JobWitness,
    // mut metadata: Metadata,
    // mut log: std::fs::File,
    // mut metadata_log: std::fs::File,
) -> Result<(), SubiloError> {
    log.write_all(project.description().as_bytes())
        .map_err(|err| SubiloError::WriteLogFile { source: err })?;

    for command in &project.commands {
        debug!("Running command: {}", &command);
        log.write_all(format!("$ {}\n", &command).as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        let path = shellexpand::tilde(&project.path).into_owned();

        match run_command(&path, &command, &log) {
            Ok(output) => match (output.status.success(), output.status.code()) {
                (true, _) => (),
                (_, Some(code)) => {
                    log.write_all(format!("Exit {}\n", code).as_bytes())
                        .map_err(|err| SubiloError::WriteLogFile { source: err })?;

                    metadata.status = MetadataStatus::Failed;
                    break;
                }
                (_, None) => {
                    log.write_all("Process terminated by signal\n".to_string().as_bytes())
                        .map_err(|err| SubiloError::WriteLogFile { source: err })?;

                    metadata.status = MetadataStatus::Failed;
                    break;
                }
            },
            Err(err) => {
                log.write_all(err.to_string().as_bytes())
                    .map_err(|err| SubiloError::WriteLogFile { source: err })?;

                metadata.status = MetadataStatus::Failed;
                break;
            }
        }
    }

    if let MetadataStatus::Started = metadata.status {
        metadata.status = MetadataStatus::Succeeded;
    }
    metadata.ended_at = Some(Utc::now().to_rfc3339());
    metadata_log
        .seek(SeekFrom::Start(0))
        .map_err(|err| SubiloError::WriteLogFile { source: err })?;
    metadata_log
        .write_all(metadata.to_json_string()?.as_bytes())
        .map_err(|err| SubiloError::WriteLogFile { source: err })?;

    Ok(())
}

pub fn spawn_job(project: Project, witness: job::JobWitness) -> Result<String, SubiloError> {
    witness.start(project.clone());

    debug!("Spawning thread to run deployment for '{}'", &project.name);
    thread::spawn(move || {
        let project_name = project.name.clone();
        let result = run_project_deployment(project, witness);

        match result {
            Ok(_) => debug!("Deployment for '{}' processed successfully", &project_name),
            Err(err) => error!(
                "Failed running deployment for '{}'.\nWith error:\n{}",
                &project_name, err
            ),
        }
    });

    Ok(job_name)
}
