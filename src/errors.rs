use actix_http::ResponseBuilder;
use actix_web::http::{header, StatusCode};
use actix_web::HttpResponse;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum ThreshError {
    #[error("Failed to read Thresh file, {}", source)]
    ReadThreshFileError { source: std::io::Error },

    #[error("Failed to create log directory, {}", source)]
    CreateLogDirError { source: std::io::Error },

    #[error("Failed to create log file, {}", source)]
    CreateLogFile { source: std::io::Error },

    #[error("Failed to write log file, {}", source)]
    WriteLogFile { source: std::io::Error },

    #[error("Failed to parse Thresh file, {}", source)]
    ParseThreshFile { source: toml::de::Error },
}

impl actix_web::error::ResponseError for ThreshError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match &self {
            ThreshError::ParseThreshFile { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            ThreshError::CreateLogDirError { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            ThreshError::ReadThreshFileError { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            ThreshError::CreateLogFile { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            ThreshError::WriteLogFile { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
