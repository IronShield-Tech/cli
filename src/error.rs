use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Challenge solving error: {0}")]
    ChallengeSolvingError(String),
    #[error("Argument parsing error: {0}")]
    ArgumentError(String),
    #[error("File operation error: {0}")]
    FileError(String),
    #[error("JSON processing error: {0}")]
    JsonError(String),
    #[error("General error: {0}")]
    General(String),
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::FileError(err.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::JsonError(err.to_string())
    }
}

impl From<clap::Error> for CliError {
    fn from(err: clap::Error) -> Self {
        CliError::ArgumentError(err.to_string())
    }
}

impl From<color_eyre::Report> for CliError {
    fn from(err: color_eyre::Report) -> Self {
        CliError::General(err.to_string())
    }
}