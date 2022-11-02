use crate::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Serialize, Debug, Error)]
pub struct ApiError {
    status_code: u16,
    errors: Vec<String>,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Err {} ", &self.status_code).to_string())
    }
}

impl ApiError {
    pub fn new(status_code: u16, err: impl ToString) -> Self {
        let mut errors: Vec<String> = Vec::new();
        errors.push(err.to_string());
        ApiError {
            status_code,
            errors,
        }
    }

    pub fn new_internal(err: impl ToString) -> Self {
        let mut errors: Vec<String> = Vec::new();
        errors.push(err.to_string());
        ApiError {
            status_code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            errors: errors,
        }
    }
    pub fn new_bad_request(err: impl ToString) -> Self {
        let mut errors: Vec<String> = Vec::new();
        errors.push(err.to_string());
        ApiError {
            status_code: StatusCode::BAD_REQUEST.as_u16(),
            errors: errors,
        }
    }

    pub fn append_error(&mut self, err: impl ToString) {
        let _ = &self.errors.push(err.to_string());
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.status_code).unwrap(),
            serde_json::to_string(&self).unwrap(),
        )
            .into_response()
    }
}
