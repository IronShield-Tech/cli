use ironshield::ClientConfig;
use ironshield::error::ErrorHandler;

pub struct ConfigManager;

#[allow(dead_code)]
impl ConfigManager {
    /// Loads and saved a default configuration file
    /// as `ironshield.toml` in the specified path.
    /// 
    /// # Arguments
    /// * `path`: The path where the default configuration 
    ///           file should be created.
    /// 
    /// # Returns
    /// * `Result<ClientConfig, ErrorHandler>`: The default configuration
    ///                                         created or an error if it fails.
    pub fn create_default_config(
        path: &str
    ) -> Result<ClientConfig, ErrorHandler> {
        let config = ClientConfig::default();
        ClientConfig::save_to_file(&config, path)?;

        println!("Created default configuration file at '{path}'");
        Ok(config)
    }

    /// Validate an existing configuration file.
    ///
    /// # Arguments
    /// * `path`: The path to the TOML configuration file.
    ///
    /// # Returns
    /// * `Result<(), ErrorHandler>`: Indication of success or failure.
    pub fn validate_config_file(path: &str) -> Result<(), ErrorHandler> {
        let content = std::fs::read_to_string(path)
            .map_err(ErrorHandler::Io)?;

        let config: ClientConfig = toml::from_str(&content)
            .map_err(|e| ErrorHandler::config_error(
                format!("Failed to parse TOML config file '{path}': {e}")
            ))?;

        config.validate()
              .map_err(|e| ErrorHandler::config_error(
                  format!("Configuration validation failed: {e}")
              ))?;

        Ok(())
    }

    /// Loads configuration from a file and applies command-line overrides.
    ///
    /// At the moment, the only override supported is the `verbose` setting.
    ///
    /// # Arguments
    /// * `path`:             Optional path to a configuration file.
    /// * `verbose_override`: Override verbose setting from the command line.
    ///
    /// # Returns
    /// * `Result<ClientConfig, ErrorHandler>`: The final configuration with overrides
    ///                                         applied.
    ///
    /// # Example
    /// ```
    /// use ironshield_cli::config::ConfigManager;
    ///
    /// // Load with verbose override.
    /// let config = ConfigManager::load_with_overrides(
    ///     Some("ironshield.toml".to_string()),
    ///     Some(true)
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load_with_overrides(
        path:             Option<String>,
        verbose_override: Option<bool>,
    ) -> Result<ClientConfig, ErrorHandler> {
        let mut config = match path {
            Some(config_path) => {
                ClientConfig::from_file(&config_path)
                    .map_err(|e| ErrorHandler::config_error(format!("Failed to load config: {e}")))?
            }
            None => {
                println!("No config file specified, using default configuration.");
                ClientConfig::default()
            }
        };
        
        if let Some(verbose) = verbose_override {
            config.set_verbose(verbose);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::time::Duration;

    #[test]
    fn test_config_roundtrip() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.toml");
        let file_path_str = file_path.to_str().unwrap();

        // Create a custom configuration.
        let mut original_config = ClientConfig::default();
        original_config.set_verbose(true);
        original_config.set_timeout(Duration::from_secs(45)).unwrap();

        // Save and reload.
        ClientConfig::save_to_file(&original_config, file_path_str).unwrap();
        let loaded_config = ClientConfig::from_file(file_path_str).unwrap();

        // Verify roundtrip accuracy.
        assert_eq!(original_config.api_base_url, loaded_config.api_base_url);
        assert_eq!(original_config.timeout, loaded_config.timeout);
        assert_eq!(original_config.verbose, loaded_config.verbose);
        assert_eq!(original_config.num_threads, loaded_config.num_threads);
        assert_eq!(original_config.user_agent, loaded_config.user_agent);
    }

    #[test]
    fn test_config_missing_file_uses_default() {
        let result = ClientConfig::from_file("nonexistent_file.toml");
        assert!(result.is_ok());

        let config = result.unwrap();
        let default_config = ClientConfig::default();
        assert_eq!(config.api_base_url, default_config.api_base_url);
    }

    #[test]
    fn test_invalid_toml_returns_error() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid_config.toml");
        let file_path_str = file_path.to_str().unwrap();

        // Write invalid TOML.
        std::fs::write(file_path_str, "invalid toml content [[[").unwrap();

        let result = ClientConfig::from_file(file_path_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_values_return_error() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid_values_config.toml");
        let file_path_str = file_path.to_str().unwrap();

        // Write TOML with invalid configuration values.
        let invalid_toml = r#"
        api_base_url = ""
        timeout = 0
        verbose = false
        "#;
        std::fs::write(file_path_str, invalid_toml).unwrap();

        let result = ClientConfig::from_file(file_path_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_default_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("default_config.toml");
        let file_path_str = file_path.to_str().unwrap();

        let config = ConfigManager::create_default_config(file_path_str).unwrap();

        // Verify the file was created and is valid.
        assert!(file_path.exists());
        let loaded_config = ClientConfig::from_file(file_path_str).unwrap();
        assert_eq!(config.api_base_url, loaded_config.api_base_url);
    }

    #[test]
    fn test_validate_config_file_valid() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("valid_config.toml");
        let file_path_str = file_path.to_str().unwrap();
        
        // Create a valid configuration file.
        let config = ClientConfig::default();
        ClientConfig::save_to_file(&config, file_path_str).unwrap();

        // Validation should succeed.
        let result = ConfigManager::validate_config_file(file_path_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_file_invalid() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("invalid_config.toml");
        let file_path_str = file_path.to_str().unwrap();

        // Write invalid TOML.
        std::fs::write(file_path_str, "invalid toml [[[").unwrap();

        // Validation should fail.
        let result = ConfigManager::validate_config_file(file_path_str);
        assert!(result.is_err());
    }
}
