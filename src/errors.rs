use actix_http::ResponseBuilder;
use actix_web::http::{header, StatusCode};
use actix_web::HttpResponse;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum ThreshError {
    #[error("Failed to read Thresh file, {}", source)]
    ReadThreshFile { source: std::io::Error },

    #[error("Failed to create log directory, {}", source)]
    CreateLogDir { source: std::io::Error },

    #[error("Failed to create log file, {}", source)]
    CreateLogFile { source: std::io::Error },

    #[error("Failed to write log file, {}", source)]
    WriteLogFile { source: std::io::Error },

    #[error("Failed to parse Thresh file, {}", source)]
    ParseThreshFile { source: toml::de::Error },

    #[error("Failed to read file name")]
    ReadFileName {},

    #[error("Failed to serialize Metadata structure to JSON, {}", source)]
    SerializeMetadataToJSON { source: serde_json::error::Error },

    #[error("Failed to execute child process, {}", source)]
    ExecuteChildProcess { source: std::io::Error },

    #[error("Failed to wait on child process, {}", source)]
    WaitOnChildProcess { source: std::io::Error },
}

impl actix_web::error::ResponseError for ThreshError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
