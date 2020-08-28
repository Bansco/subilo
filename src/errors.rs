use actix_http::ResponseBuilder;
use actix_web::http::{header, StatusCode};
use actix_web::HttpResponse;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum SubiloError {
    #[error("Failed to execute spawned blocking thread")]
    JoinHandle { source: tokio::task::JoinError },

    #[error("Failed to read application Context")]
    ReadContext {},

    #[error("Failed to read Subilo rc file, {}", source)]
    ReadSubiloRC { source: std::io::Error },

    #[error("Failed to parse Subilo rc file, {}", source)]
    ParseSubiloRC { source: toml::de::Error },

    #[error("Failed to create log directory, {}", source)]
    CreateLogDir { source: std::io::Error },

    #[error("Failed to create log file, {}", source)]
    CreateLogFile { source: std::io::Error },

    #[error("Failed to write log file, {}", source)]
    WriteLogFile { source: std::io::Error },

    #[error("Failed to authenticate request, {}", source)]
    Authenticate { source: jsonwebtoken::errors::Error },

    #[error("Token missing")]
    MissingToken {},

    #[error("Failed to parse project commands to JSON format")]
    ParseProjectCommands { source: serde_json::error::Error },

    #[error("Failed to execute database query, {}", source)]
    DatabaseQuery { source: rusqlite::Error },

    #[error("Failed to communicate with database actor, {}", source)]
    DatabaseActor { source: actix::MailboxError },
}

impl actix_web::error::ResponseError for SubiloError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match &self {
            SubiloError::Authenticate { source: _ } => StatusCode::UNAUTHORIZED,
            SubiloError::MissingToken {} => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
