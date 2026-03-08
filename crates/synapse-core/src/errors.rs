use thiserror::Error;

#[derive(Debug, Error)]
pub enum SynapseError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Type error at {location}: {message}")]
    Type { message: String, location: String },

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, SynapseError>;
