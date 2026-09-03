use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("room not found")]
    RoomNotFound,

    #[error("player not found — join the room before checking in")]
    PlayerNotFound,

    #[error("already checked in for today")]
    AlreadyCheckedIn,

    #[error("the {0}-day week for this room is already complete")]
    WeekComplete(i32),

    #[error("unknown activity: {0}")]
    UnknownActivity(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("image error")]
    Image(#[from] image::ImageError),

    #[error("multipart error")]
    Multipart(#[from] actix_multipart::MultipartError),
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::RoomNotFound | AppError::PlayerNotFound => StatusCode::NOT_FOUND,
            AppError::AlreadyCheckedIn | AppError::WeekComplete(_) => StatusCode::CONFLICT,
            AppError::UnknownActivity(_) | AppError::BadRequest(_) | AppError::Multipart(_) => {
                StatusCode::BAD_REQUEST
            }
            AppError::Database(_) | AppError::Io(_) | AppError::Image(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_response(&self) -> HttpResponse {
        // Log the real cause server-side, but never leak internal details
        // (DB errors, file paths, etc.) to the client.
        if self.status_code() == StatusCode::INTERNAL_SERVER_ERROR {
            match self {
                AppError::Database(e) => tracing::error!(error = %e, "internal database error"),
                AppError::Io(e) => tracing::error!(error = %e, "internal io error"),
                AppError::Image(e) => tracing::error!(error = %e, "internal image error"),
                _ => tracing::error!(error = %self, "internal error"),
            }
        }
        HttpResponse::build(self.status_code()).json(json!({ "error": self.to_string() }))
    }
}

pub type AppResult<T> = Result<T, AppError>;
