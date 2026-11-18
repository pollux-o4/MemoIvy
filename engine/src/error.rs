use crate::domain::Snapshot;

#[derive(Clone, Debug)]
pub(crate) struct ApiError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) snapshot: Option<Box<Snapshot>>,
}
pub(crate) type ApiResult = Result<serde_json::Value, ApiError>;

pub(crate) fn internal(e: impl std::fmt::Display) -> ApiError {
    err("INTERNAL", e.to_string())
}
pub(crate) fn err(code: &str, message: impl Into<String>) -> ApiError {
    ApiError {
        code: code.into(),
        message: message.into(),
        snapshot: None,
    }
}
