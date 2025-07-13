use std::time::Duration;

pub struct ClientConfig {
    pub endpoint:     String,
    pub api_base_url: String,
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