use crate::core;
use crate::core::Metadata;
use crate::core::MetadataStatus;
use crate::database;
use crate::Context;
use crate::SubiloError;
use actix::prelude::*;
use actix_web::dev::Payload;
use actix_web::HttpRequest;
use chrono::Utc;
use futures::future;
use std::fs;
use std::io::Write;

pub struct JobWitness {
    database: Addr<database::Database>,
    // Probably we should deref to actual value
    context: actix_web::web::Data<Context>,
    project: Option<core::Project>,
}

impl JobWitness {
    pub fn start(&self, project: core::Project) -> Result<(), SubiloError> {
        self.project = Some(project);

        let job_name = create_job_name(&project.name);
        let file_name = create_log_name(&job_name, &self.context.logs_dir);
        let metadata_file_name = create_metadata_log_name(&job_name, &self.context.logs_dir);

        fs::create_dir_all(&self.context.logs_dir)
            .map_err(|err| SubiloError::CreateLogDir { source: err })?;

        let metadata = Metadata {
            name: project.name.clone(),
            status: MetadataStatus::Started,
            started_at: Utc::now().to_rfc3339(),
            ended_at: None,
        };

        let log = fs::File::create(file_name)
            .map_err(|err| SubiloError::CreateLogFile { source: err })?;

        let mut metadata_log = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(metadata_file_name)
            .map_err(|err| SubiloError::CreateLogFile { source: err })?;

        metadata_log
            .write_all(metadata.to_json_string()?.as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        Ok(())
    }
}

// db.do_send(database::NewJob {
//     id: "123".to_owned(),
//     name: "test".to_owned(),
// });

impl actix_web::FromRequest for JobWitness {
    type Config = ();
    type Error = SubiloError;
    type Future = future::Ready<Result<Self, SubiloError>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let context = req
            .app_data::<actix_web::web::Data<Context>>()
            .unwrap()
            .clone();

        let database = req
            .app_data::<actix_web::web::Data<Addr<database::Database>>>()
            .unwrap()
            .get_ref()
            .clone();

        let this = Self {
            database,
            context,
            project: None,
        };

        future::ok(this)
    }
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
