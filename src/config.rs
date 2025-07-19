use serde::{
    Deserialize,
    Serialize
};

use crate::error::CliError;

use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub endpoint:     String,
    pub api_base_url: String,
    #[serde(with = "duration_serde")]
    pub timeout:      Duration,
    pub verbose:      bool,
    pub num_threads:  Option<usize>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint:     "https://example.com/api".to_string(),
            api_base_url: "https://api.ironshield.cloud".to_string(),
            timeout:      Duration::from_secs(30),
            verbose:      true,
            num_threads:  None, // Default to single-threaded.
        }
    }
}

impl ClientConfig {
    pub fn from_file(path: &str) -> Result<Self, CliError> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                toml::from_str(&content)
                    .map_err(|e| CliError::ConfigError(format!("Failed to parse config file: {}", e)))
            }
            Err(_) => { // File doesn't exist, use default configuration.
                eprintln!("Config file '{}' not found, using default configuration.", path);
                Ok(Self::default())
            }
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), CliError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| CliError::ConfigError(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| CliError::FileError(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}

mod duration_serde {
    use serde::{
        Deserialize,
        Deserializer,
        Serializer
    };
    use std::time::Duration;

    pub fn serialize<S>(
        duration: &Duration,
        serializer: S
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(
        deserializer: D
    ) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}