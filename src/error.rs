use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Solving error: {0}")]
    SolvingError(String),
    #[error("Invalid solution: {0}")]
    InvalidSolution(String),
    #[error("TUI error: {0}")]
    TuiError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IronShield API error: {0}")]
    IronShieldApi(#[from] ironshield_api::handler::error::ErrorHandler),
}