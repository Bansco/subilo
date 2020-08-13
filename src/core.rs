use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{str, thread};

use crate::errors::SubiloError;
use crate::job;
use crate::Context;

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub commands: Vec<String>,
}

impl Project {
    pub fn description(&self) -> String {
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
    witness: &job::Witness,
) -> Result<std::process::Output, RunError> {
    let stdout = witness
        .try_clone_log()
        .map_err(|err| RunError::CloneLogFile { source: err })?;
    let stderr = witness
        .try_clone_log()
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
    mut witness: job::Witness,
) -> Result<(), SubiloError> {
    for command in &project.commands {
        debug!("Running command: {}", &command);

        witness.report_command(&command)?;

        let path = shellexpand::tilde(&project.path).into_owned();

        match run_command(&path, &command, &witness) {
            Ok(output) => {
                if output.status.success() {
                    witness.report_command_success()?
                } else {
                    witness.report_command_error_by_code(output.status.code())?;
                    break;
                }
            }
            Err(err) => {
                witness.report_command_error(err)?;
                break;
            }
        }
    }

    Ok(())
}

pub fn spawn_job(project: Project, ctx: Context) -> Result<String, SubiloError> {
    let job_name = create_job_name(&project.name);
    let witness = job::Witness::new(job_name.clone(), project.clone(), ctx)?;

    debug!(
        "Spawning thread to run deployment for project {}",
        &project.name
    );
    thread::spawn(move || {
        let project_name = project.name.clone();
        let result = run_project_deployment(project, witness);

        match result {
            Ok(_) => debug!(
                "Deployment for project {} processed successfully",
                project_name
            ),
            Err(err) => error!(
                "Failed running deployment for project {}.\nWith error:\n{}",
                project_name, err
            ),
        }
    });

    Ok(job_name)
}

pub fn create_job_name(repository: &str) -> String {
    let repository = repository.replace("/", "-");
    let now = Utc::now().format("%Y-%m-%d--%H-%M-%S").to_string();
    format!("{}_{}", repository, now)
}
