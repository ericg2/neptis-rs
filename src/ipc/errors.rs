use rocket::http::Status;
use rocket::response::{status, Responder};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("Internal Error: {0}")]
    InternalError(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
    
    #[error(transparent)]
    Sql(#[from] sqlx::Error),

    #[error("A timeout error has occurred.")]
    Timeout,
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let status = match self {
            ApiError::InternalError(_) => Status::InternalServerError,
            ApiError::BadRequest(_) => Status::BadRequest,
            ApiError::Unauthorized(_) => Status::Unauthorized,
            ApiError::Timeout => Status::RequestTimeout,
            ApiError::IoError(_) => Status::InternalServerError,
            ApiError::Reqwest(_) => Status::InternalServerError,
            ApiError::Archive(_) => Status::InternalServerError,
            ApiError::Sql(_) => Status::InternalServerError,
        };
        status::Custom(status, json!({"error": self.to_string()})).respond_to(request)
    }
}