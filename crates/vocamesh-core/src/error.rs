use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceError {
    pub code: &'static str,
    pub message: &'static str,
}

impl ServiceError {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}
