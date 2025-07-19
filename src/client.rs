use reqwest::Client;

use ironshield_api::handler::{
    error::ErrorHandler,
    result::ResultHandler
};
use ironshield_types::{
    chrono,
    IronShieldChallenge,
    IronShieldRequest,
};

use crate::config::ClientConfig;
use crate::http_builder::HttpClientBuilder;
use crate::response::ApiResponse;

use std::time::Instant;

pub struct IronShieldClient {
    config:      ClientConfig,
    http_client: Client,
}

impl IronShieldClient {
    /// Creates a new IronShield client with the provided configuration.
    ///
    /// # Arguments
    /// * `config`: The client configuration.
    ///
    /// # Return
    /// * `ResultHandler<Self>`: The initialized client or an error.
    ///
    /// # Example
    /// ```
    /// let config = ClientConfig::from_file("ironshield.toml")?;
    /// let client = IronShieldClient::new(config)?;
    /// ```
    pub fn new(config: ClientConfig) -> ResultHandler<Self> {
        crate::verbose_section!(config, "Client Initialization");

        if !config.endpoint.starts_with("https://") {
            return Err(ErrorHandler::config_error(
                ironshield_api::handler::error::INVALID_ENDPOINT
            ));
        }

        let http_client = HttpClientBuilder::new()
            .timeout(config.timeout)
            .build()?;

        crate::verbose_log!(config, success, "Client initialized successfully.");

        Ok(Self {
            config,
            http_client
        })
    }

    /// Fetches a challenge from the IronShield API.
    ///
    /// # Arguments
    /// * `endpoint`: The protected endpoint URL to access.
    ///
    /// # Returns
    /// * `ResultHandler<IronShieldChallenge>`: The challenge to solve.
    ///
    /// # Examples
    /// ```
    /// let challenge = client.fetch_challenge("https://example.com/protected").await?;
    /// println!("Challenge difficulty: {}", challenge.recommended_attempts);
    /// ```
    pub async fn fetch_challenge(
        &self,
        endpoint: &str
    ) -> ResultHandler<IronShieldChallenge> {
        crate::verbose_section!(self.config, "Challenge Fetching");
        crate::verbose_log!(self.config, network, "Requesting challenge for endpoint: {}", endpoint);

        let request = IronShieldRequest::new(
            endpoint.to_string(),
            chrono::Utc::now().timestamp_millis(),
        );

        let start_time = Instant::now();

        let response = self.make_api_request(&request).await?;

        crate::verbose_log!(
            self.config,
            timing,
            "Challenge fetch completed in {:?}",
            start_time.elapsed()
        );

        let api_response = ApiResponse::from_json(response)?;
        crate::verbose_log!(self.config, info, "API response: {}", api_response.message);

        api_response.extract_challenge()
    }

    /// Makes a standardized API request to the IronShield API service.
    ///
    /// # Arguments
    /// * `request`: The request payload to send to the API.
    ///
    /// # Returns
    /// * `ResultHandler<serde_json::Value>`: The parsed JSON response
    ///                                       or an error if the
    ///                                       request fails.
    async fn make_api_request(
        &self,
        request: &IronShieldRequest
    ) -> ResultHandler<serde_json::Value> {
        crate::verbose_log!(
            self.config,
            network,
            "Making API request to: {}/request",
            self.config.api_base_url
        );

        // Serialize the request to JSON for logging.
        match serde_json::to_string_pretty(request) {
            Ok(json_string) => {
                crate::verbose_log!(
                    self.config,
                    submit,
                    "Request JSON payload:\n{}",
                    json_string
                );
            }
            Err(e) => {
                crate::verbose_log!(
                    self.config,
                    warning,
                    "Failed to serialize request for logging: {}",
                    e
                );
            }
        }

        let response = self
            .http_client
            .post(&format!("{}/request", self.config.api_base_url))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(ErrorHandler::from_network_error)?;

        crate::verbose_log!(
            self.config,
            network,
            "API response status: {}",
            response.status()
        );

        if !response.status().is_success() {
            return Err(ErrorHandler::ProcessingError(format!(
                "API request failed with status: {}",
                response.status()
            )))
        }

        let json_response = response.json().await.map_err(ErrorHandler::from_network_error)?;

        // Log the complete response JSON.
        match serde_json::to_string_pretty(&json_response) {
            Ok(response_json) => {
                crate::verbose_log!(
                    self.config,
                    receive,
                    "Response JSON payload:\n{}",
                    response_json
                );
            }
            Err(e) => {
                crate::verbose_log!(
                    self.config,
                    warning,
                    "Failed to serialize response for logging: {}",
                    e
                );
            }
        }

        Ok(json_response)
    }
}