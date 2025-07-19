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
use crate::{verbose_log, verbose_section};
use crate::http_builder::HttpClientBuilder;
use crate::response::ApiResponse;

use std::time::Instant;

pub struct IronShieldClient {
    config: ClientConfig,
    http_client: Client,
}

impl IronShieldClient {
    pub fn new(config: ClientConfig) -> ResultHandler<Self> {
        verbose_section!(config, "Client Initialization");

        if !config.endpoint.starts_with("https://") {
            return Err(ErrorHandler::config_error(
                ironshield_api::handler::error::INVALID_ENDPOINT
            ));
        }

        let http_client = HttpClientBuilder::new()
            .timeout(config.timeout)
            .build()?;

        verbose_log!(config, success, "Client initialized successfully.");

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
    pub async fn fetch_challenge(
        &self,
        endpoint: &str
    ) -> ResultHandler<IronShieldChallenge> {
        verbose_section!(self.config, "Challenge Fetching");
        verbose_log!(self.config, network, "Requesting challenge for endpoint: {}", endpoint);

        let request = IronShieldRequest::new(
            endpoint.to_string(),
            chrono::Utc::now().timestamp_millis(),
        );

        let start_time = Instant::now();

        let response = self.make_api_request(&request).await?;

        verbose_log!(
            self.config,
            timing,
            "Challenge fetch completed in {:?}",
            start_time.elapsed()
        );

        let api_response = ApiResponse::from_json(response)?;
        verbose_log!(self.config, info, "API response: {}", api_response.message);

        api_response.extract_challenge()
    }

    async fn make_api_request(
        &self,
        request: &IronShieldRequest
    ) -> ResultHandler<serde_json::Value> {
        let response = self
            .http_client
            .post(&format!("{}/request", self.config.api_base_url))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(ErrorHandler::from_network_error)?;

        if !response.status().is_success() {
            return Err(ErrorHandler::ProcessingError(format!(
                "API request failed with status: {}",
                response.status()
            )))
        }

        response.json().await.map_err(ErrorHandler::from_network_error)
    }
}
