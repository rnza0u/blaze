use std::fmt::Display;

use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Any(Box<dyn std::error::Error>),
    BadParams,
    Configuration(&'static str),
    InvalidState(&'static str),
    VersionNotFound,
}

impl Error {
    pub fn any<E>(error: E) -> Self
    where
        E: std::error::Error + 'static,
    {
        Self::Any(Box::new(error))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(reason) => write!(f, "configuration error: {reason}"),
            Self::VersionNotFound => f.write_str("version does not exist"),
            Self::Any(error) => error.fmt(f),
            Self::InvalidState(reason) => write!(f, "invalid server state: {reason}"),
            Self::BadParams => f.write_str("invalid request"),
        }
    }
}

impl std::error::Error for Error {}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::VersionNotFound => StatusCode::NOT_FOUND,
            Self::BadParams => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let status = self.status_code();
        let mut response_builder = HttpResponse::build(status);

        if status.is_server_error() {
            response_builder.finish()
        } else {
            response_builder.body(self.to_string())
        }
    }
}
