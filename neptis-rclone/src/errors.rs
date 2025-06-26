use rocket::http::Status;
use rocket::response::{status, Responder};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidateError {
    #[error("A value is requred: {0}")]
    ValueRequired(String),

    #[error("A value is out of range: {0}")]
    OutOfRange(String),

    #[error("A value has a bad combination: {0}")]
    BadCombo(String),

    #[error("A generic error has occurred: {0}")]
    CustomError(String),
}

#[derive(Debug, Error)]
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
    Validation(#[from] ValidateError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
    
    #[error(transparent)]
    Sql(#[from] sqlx::Error),

    #[error("A timeout error has occurred.")]
    Timeout,
}

impl ApiError {
    pub fn enum_not_found(msg: String) -> Self {
        Self::InternalError(msg)
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let status = match self {
            ApiError::InternalError(_) => Status::InternalServerError,
            ApiError::BadRequest(_) => Status::BadRequest,
            ApiError::Unauthorized(_) => Status::Unauthorized,
            ApiError::Validation(_) => Status::BadRequest,
            ApiError::Timeout => Status::RequestTimeout,
            ApiError::IoError(_) => Status::InternalServerError,
            ApiError::Reqwest(_) => Status::InternalServerError,
            ApiError::Archive(_) => Status::InternalServerError,
            ApiError::Sql(_) => Status::InternalServerError,
        };
        status::Custom(status, json!({"error": self.to_string()})).respond_to(request)
    }
}