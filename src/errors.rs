use actix_http::ResponseBuilder;
use actix_web::http::{header, StatusCode};
use actix_web::HttpResponse;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum SubiloError {
    #[error("Failed to read Subilo file, {}", source)]
    ReadSubiloFile { source: std::io::Error },

    #[error("Failed to parse Subilo file, {}", source)]
    ParseSubiloFile { source: toml::de::Error },

    #[error("Failed to create log directory, {}", source)]
    CreateLogDir { source: std::io::Error },

    #[error("Failed to create log file, {}", source)]
    CreateLogFile { source: std::io::Error },

    #[error("Failed to write log file, {}", source)]
    WriteLogFile { source: std::io::Error },

    #[error("Failed to clone log file, {}", source)]
    CloneLogFile { source: std::io::Error },

    #[error("Failed to read file name")]
    ReadFileName {},

    #[error("Failed to serialize Metadata structure to JSON, {}", source)]
    SerializeMetadataToJSON { source: serde_json::error::Error },

    #[error("Failed to execute command with child process, {}", source)]
    ExecuteCommand { source: std::io::Error },

    #[error("Failed to authenticate request, {}", source)]
    Authenticate { source: jsonwebtoken::errors::Error },
}

impl actix_web::error::ResponseError for SubiloError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
