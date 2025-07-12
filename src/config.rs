use std::time::Duration;

pub struct ClientConfig {
    pub endpoint:     String,
    pub api_base_url: String,
    pub timeout:      Duration,
    pub verbose:      bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            endpoint:     "https://example.com/api".to_string(),
            api_base_url: "https://api.ironshield.cloud".to_string(),
            timeout:      Duration::from_secs(30),
            verbose:      false,
        }
    }
}