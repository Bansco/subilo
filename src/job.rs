use crate::core;
use crate::database;
use crate::Context;
use crate::SubiloError;
use futures::executor::block_on;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

pub const CREATE_JOB_TABLE_QUERY: &str = "
    CREATE TABLE IF NOT EXISTS jobs (
        id TEXT PRIMARY KEY NOT NULL,
        name TEXT NOT NULL UNIQUE,
        status TEXT NOT NULL,
        started_at TEXT NOT NULL,
        ended_at TEXT
    )
";

pub const INSERT_JOB_QUERY: &str = "
    INSERT INTO jobs (id, name, status, started_at)
    VALUES (?1, ?2, ?3, ?4)
";

pub const UPDATE_JOB_QUERY: &str = "
    UPDATE jobs
    SET status = ?2, ended_at = ?3
    WHERE id = ?1
";

pub const GET_ALL_JOBS_QUERY: &str = "
    SELECT id, name, status, started_at, ended_at
    FROM jobs
";

pub const GET_JOB_BY_ID_QUERY: &str = "
    SELECT id, name, status, started_at, ended_at
    FROM jobs
    WHERE id = ?1
";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
    Started,
    Succeeded,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: String,
}

pub struct Witness {
    id: String,
    log: std::fs::File,
    // TODO: Consider deref to actual value
    context: actix_web::web::Data<Context>,
}

impl Witness {
    pub fn new(
        job_name: String,
        project: core::Project,
        context: actix_web::web::Data<Context>,
    ) -> Result<Self, SubiloError> {
        fs::create_dir_all(&context.logs_dir)
            .map_err(|err| SubiloError::CreateLogDir { source: err })?;

        let mut log = fs::File::create(create_log_name(&job_name, &context.logs_dir))
            .map_err(|err| SubiloError::CreateLogFile { source: err })?;

        log.write_all(&project.description().as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        let id = nanoid!();
        let status = JobStatus::Started.to_string().to_lowercase();
        let started_at = now();

        let insert_job = context.database.send(database::Execute {
            query: INSERT_JOB_QUERY.to_owned(),
            params: vec![id.clone(), job_name, status, started_at],
        });

        block_on(insert_job)
            .map_err(|err| SubiloError::DatabaseActor { source: err })?
            .map_err(|err| SubiloError::Database { source: err })?;

        Ok(Self { id, context, log })
    }

    pub fn report_command(&mut self, command: &str) -> Result<(), SubiloError> {
        self.log
            .write_all(format!("$ {}\n", &command).as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })
    }

    pub fn report_command_success(&self) -> Result<(), SubiloError> {
        let ended_at = now();
        let status = JobStatus::Succeeded.to_string().to_lowercase();

        let update_job = self.context.database.send(database::Execute {
            query: UPDATE_JOB_QUERY.to_owned(),
            params: vec![self.id.clone(), status, ended_at],
        });

        block_on(update_job)
            .map_err(|err| SubiloError::DatabaseActor { source: err })?
            .map_err(|err| SubiloError::Database { source: err })
            .map(|_res| ())
    }

    pub fn report_command_error_by_code(
        &mut self,
        status_code: Option<i32>,
    ) -> Result<(), SubiloError> {
        match status_code {
            Some(code) => self
                .log
                .write_all(format!("Exit {}\n", code).as_bytes())
                .map_err(|err| SubiloError::WriteLogFile { source: err })?,
            None => self
                .log
                .write_all("Process terminated by signal\n".to_string().as_bytes())
                .map_err(|err| SubiloError::WriteLogFile { source: err })?,
        };

        let ended_at = now();
        let status = JobStatus::Failed.to_string().to_lowercase();

        let update_job = self.context.database.send(database::Execute {
            query: UPDATE_JOB_QUERY.to_owned(),
            params: vec![self.id.clone(), status, ended_at],
        });

        block_on(update_job)
            .map_err(|err| SubiloError::DatabaseActor { source: err })?
            .map_err(|err| SubiloError::Database { source: err })
            .map(|_res| ())
    }

    pub fn report_command_error(&mut self, err: core::RunError) -> Result<(), SubiloError> {
        self.log
            .write_all(err.to_string().as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        let ended_at = now();
        let status = JobStatus::Failed.to_string().to_lowercase();

        let update_job = self.context.database.send(database::Execute {
            query: UPDATE_JOB_QUERY.to_owned(),
            params: vec![self.id.clone(), status, ended_at],
        });

        block_on(update_job)
            .map_err(|err| SubiloError::DatabaseActor { source: err })?
            .map_err(|err| SubiloError::Database { source: err })
            .map(|_res| ())
    }

    pub fn try_clone_log(&self) -> Result<std::fs::File, std::io::Error> {
        self.log.try_clone()
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn create_log_name(job: &str, log_dir: &str) -> String {
    let log_dir = shellexpand::tilde(&log_dir).into_owned();
    format!("{}/{}.log", log_dir, job)
}
