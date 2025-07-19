use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    /// Configuration-related errors
    /// (invalid settings, missing files, etc.).
    #[error("Configuration error: {0}")]
    Config(String),
    /// Network communication errors from the HTTP client.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    /// API-specific errors with status
    /// code and message from the server.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code returned by the API.
        status: u16,
        /// Error message from the API response.
        message: String
    },
    /// Challenge processing errors
    /// (solving, verification, etc.).
    #[error("Challenge processing error: {0}")]
    Challenge(String),
    /// JSON parsing and serialization errors.
    #[error("Parsing error: {0}")]
    Parse(#[from] serde_json::Error),
    /// File system and I/O errors.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML configuration file parsing errors.
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
}

impl CliError {
    pub fn api_error(status: u16, message: impl Into<String>) -> Self {
        Self::Api { status, message: message.into() }
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    pub fn challenge_error(message: impl Into<String>) -> Self {
        Self::Challenge(message.into())
    }
}