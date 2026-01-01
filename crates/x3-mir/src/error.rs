use thiserror::Error;

#[derive(Debug, Error)]
#[error("{message}")]
pub struct MirError {
    message: String,
}

impl MirError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
