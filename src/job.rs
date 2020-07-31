use crate::core;
use crate::database;
use crate::Context;
use crate::SubiloError;
use chrono::Utc;
use nanoid::nanoid;
use std::fs;
use std::io::Write;

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

        let mut log =
            fs::File::create(job_name).map_err(|err| SubiloError::CreateLogFile { source: err })?;

        log.write_all(&project.description().as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        let id = nanoid!();

        context.database.do_send(database::NewJob {
            id: id.clone(),
            name: "test".to_owned(),
            started_at: Utc::now().to_rfc3339(),
        });

        Ok(Self { id, context, log })
    }

    pub fn report_command(&mut self, command: &str) -> Result<(), SubiloError> {
        self.log
            .write_all(format!("$ {}\n", &command).as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        Ok(())
    }

    pub fn report_command_success(&self) -> Result<(), SubiloError> {
        self.context.database.do_send(database::UpdateJob {
            id: self.id.clone(),
            status: "succeeded".to_owned(),
            ended_at: Utc::now().to_rfc3339(),
        });

        Ok(())
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

        self.context.database.do_send(database::UpdateJob {
            id: self.id.clone(),
            status: "failed".to_owned(),
            ended_at: Utc::now().to_rfc3339(),
        });

        Ok(())
    }

    pub fn report_command_error(&mut self, err: core::RunError) -> Result<(), SubiloError> {
        self.log
            .write_all(err.to_string().as_bytes())
            .map_err(|err| SubiloError::WriteLogFile { source: err })?;

        self.context.database.do_send(database::UpdateJob {
            id: self.id.clone(),
            status: "failed".to_owned(),
            ended_at: Utc::now().to_rfc3339(),
        });

        Ok(())
    }

    pub fn try_clone_log(&self) -> Result<std::fs::File, std::io::Error> {
        let log = self.log.try_clone()?;

        Ok(log)
    }
}
