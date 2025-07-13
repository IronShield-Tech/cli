use std::time::Instant;
use reqwest::{Client, header::HeaderMap};

use ironshield_api::handler::{error, error::ErrorHandler, result::ResultHandler};
use ironshield_types::{chrono, IronShieldChallenge, IronShieldRequest};
use crate::config::ClientConfig;

// In the very unfortunate event that the user doesn't have multi-threading support.
use ironshield_core::{find_solution_single_threaded, verify_ironshield_solution};

// In the very fortunate event that the user does have multi-threading support.
#[cfg(all(feature = "parallel", not(feature = "no-parallel")))]
use ironshield_core::{find_solution_multi_threaded};
use crate::{verbose_kv, verbose_log, verbose_section};

const USER_AGENT: &str = "curl/8.4.0";

pub struct IronShieldClient {
    config:      ClientConfig,
    http_client: Client,
}

impl IronShieldClient {
    pub fn new(config: ClientConfig) -> ResultHandler<Self> {
        if config.endpoint.is_empty() {
            return Err(ErrorHandler::config_error("Endpoint cannot be empty."));
        }

        if !config.endpoint.starts_with("https://") {
            return Err(ErrorHandler::config_error(error::INVALID_ENDPOINT));
        }

        let http_client = Client::builder()
            .timeout(config.timeout)
            .user_agent(USER_AGENT) // I picked random numbers
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(ErrorHandler::from_network_error)?;

        Ok(Self {
            config,
            http_client
        })
    }

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

        verbose_kv!(self.config, "Request Timestamp", request.timestamp);

        let start_time = Instant::now();

        let response = self
            .http_client
            .post(&format!("{}/request", self.config.api_base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(ErrorHandler::from_network_error)?;

        let fetch_duration = start_time.elapsed();
        verbose_log!(self.config, timing, "Challenge fetch completed in {:?}", fetch_duration);

        if !response.status().is_success() {
            verbose_log!(self.config, error, "API request failed with status: {}", response.status());
            return Err(ErrorHandler::ProcessingError(format!(
                "API request failed with status: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let api_response: serde_json::Value = response.json().await
            .map_err(ErrorHandler::from_network_error)?;

        let status = api_response.get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN");

        if status != "OK" {
            let error_message = api_response.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            verbose_log!(self.config, error, "API returned error: {}", error_message);
            return Err(ErrorHandler::ProcessingError(format!(
                "API returned error: {}",
                error_message
            )));
        }

        // Extract and deserialize the challenge.
        let challenge = api_response.get("challenge")
            .ok_or_else(|| {
                ErrorHandler::ProcessingError(
                    "No challenge field in API response".to_string()
                )
            })?;

        let challenge: IronShieldChallenge = serde_json::from_value(challenge.clone())
            .map_err(|e| ErrorHandler::ProcessingError(format!(
                "Failed to deserialize challenge: {}", e
            )))?;

        verbose_log!(self.config, success, "Challenge received successfully");
        verbose_kv!(self.config, "Challenge Endpoint", &challenge.website_id);
        verbose_kv!(self.config, "Expiration Time", challenge.expiration_time);

        Ok(challenge)
    }
}