use reqwest::Client;
use std::time::Instant;

use crate::config::ClientConfig;
use ironshield_api::handler::{error, error::ErrorHandler, result::ResultHandler};
use ironshield_types::{chrono, IronShieldChallenge, IronShieldChallengeResponse, IronShieldRequest};

use crate::{verbose_kv, verbose_log, verbose_section};
// In the very fortunate event that the user does have multi-threading support.
#[cfg(all(feature = "parallel", not(feature = "no-parallel")))]
use ironshield_core::find_solution_multi_threaded;
use ironshield_core::{find_solution_single_threaded, verify_ironshield_solution};
use reqwest::header::HeaderMap;

const USER_AGENT: &str = "curl/8.4.0";

pub struct IronShieldClient {
    config:      ClientConfig,
    http_client: Client,
}

impl IronShieldClient {
    pub fn new(config: ClientConfig) -> ResultHandler<Self> {
        verbose_section!(config, "Client Initialization");

        // Validate configuration using the API's error handling.
        if config.endpoint.is_empty() {
            return Err(ErrorHandler::config_error("Endpoint cannot be empty"));
        }

        if !config.endpoint.starts_with("https://") && !config.endpoint.starts_with("http://") {
            return Err(ErrorHandler::config_error("Endpoint must be a valid URL"));
        }

        verbose_kv!(config, "API Base URL", &config.api_base_url);
        verbose_kv!(config, "Timeout", format!("{:?}", config.timeout));
        verbose_kv!(config, "Threading", match config.num_threads {
            Some(n) => format!("{} threads", n),
            None => "Single-threaded".to_string(),
        });

        let http_client = Client::builder()
            .timeout(config.timeout)
            .user_agent("ironshield-cli/1.0.0")
            .danger_accept_invalid_certs(false) // Ensure SSL validation.
            .build()
            .map_err(ErrorHandler::from_network_error)?;

        verbose_log!(config, success, "Client initialized successfully");

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Fetches a challenge from the IronShield API.
    ///
    /// # Arguments
    /// * `endpoint` - The protected endpoint URL to access.
    ///
    /// # Returns
    /// * `ResultHandler<IronShieldChallenge>` - The challenge to solve.
    pub async fn fetch_challenge(&self, endpoint: &str) -> ResultHandler<IronShieldChallenge> {
        verbose_section!(self.config, "Challenge Fetching");
        verbose_log!(self.config, network, "Requesting challenge for endpoint: {}", endpoint);

        // Create the request payload.
        let request = IronShieldRequest::new(
            endpoint.to_string(),
            chrono::Utc::now().timestamp_millis(),
        );

        verbose_kv!(self.config, "Request Timestamp", request.timestamp);

        let start_time = Instant::now();

        // Send POST request to the /request endpoint.
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

        // Parse the JSON response using serde_json::Value to match the API's response format.
        let api_response: serde_json::Value = response.json().await
            .map_err(ErrorHandler::from_network_error)?;

        // Extract status from the response.
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

    /// Solves the given IronShield challenge using ironshield-core's optimized proof-of-work.
    ///
    /// This uses the production-grade solving algorithm from ironshield-core instead of
    /// a custom implementation, ensuring consistency with the broader ecosystem.
    ///
    /// # Arguments
    /// * `challenge` - The challenge to solve.
    ///
    /// # Returns
    /// * `ResultHandler<IronShieldChallengeResponse>` - The solution response.
    pub async fn solve_challenge(
        &self,
        challenge: &IronShieldChallenge,
    ) -> ResultHandler<IronShieldChallengeResponse> {
        verbose_section!(self.config, "Challenge Solving");
        verbose_log!(self.config, compute, "Starting proof-of-work solving using ironshield-core");

        verbose_kv!(self.config, "Challenge Difficulty", format!("{:?}", challenge.challenge_param));
        verbose_kv!(self.config, "Endpoint", &challenge.website_id);
        verbose_kv!(self.config, "Expires At", challenge.expiration_time);

        match self.config.num_threads {
            Some(threads) => verbose_kv!(self.config, "Solving Strategy", format!("Multi-threaded ({} threads)", threads)),
            None => verbose_kv!(self.config, "Solving Strategy", "Single-threaded"),
        }

        let start_time = Instant::now();

        // Check if challenge is already expired.
        let current_time = chrono::Utc::now().timestamp_millis();
        if current_time > challenge.expiration_time {
            verbose_log!(self.config, error, "Challenge has already expired");
            return Err(ErrorHandler::challenge_solving_error(
                "Challenge has already expired"
            ));
        }

        // Choose solving strategy based on configuration.
        let response = if let Some(num_threads) = self.config.num_threads {
            // Use multi-threaded solving if available and requested.
            #[cfg(all(feature = "parallel", not(feature = "no-parallel")))]
            {
                verbose_log!(self.config, compute, "Using parallel solving algorithm");
                tokio::task::spawn_blocking({
                    let challenge = challenge.clone();
                    move || find_solution_multi_threaded(&challenge, Some(num_threads), None, None)
                })
                    .await
                    .map_err(|e| ErrorHandler::challenge_solving_error(format!("Task execution failed: {}", e)))?
                    .map_err(|e| ErrorHandler::challenge_solving_error(e))?
            }

            #[cfg(not(all(feature = "parallel", not(feature = "no-parallel"))))]
            {
                verbose_log!(self.config, warning, "Multi-threaded solving not available, falling back to single-threaded");
                tokio::task::spawn_blocking({
                    let challenge = challenge.clone();
                    move || find_solution_single_threaded(&challenge)
                })
                    .await
                    .map_err(|e| ErrorHandler::challenge_solving_error(format!("Task execution failed: {}", e)))?
                    .map_err(|e| ErrorHandler::challenge_solving_error(e))?
            }
        } else {
            // Use single-threaded solving.
            verbose_log!(self.config, compute, "Using single-threaded solving algorithm");
            tokio::task::spawn_blocking({
                let challenge = challenge.clone();
                move || find_solution_single_threaded(&challenge)
            })
                .await
                .map_err(|e| ErrorHandler::challenge_solving_error(format!("Task execution failed: {}", e)))?
                .map_err(|e| ErrorHandler::challenge_solving_error(e))?
        };

        let solve_duration = start_time.elapsed();

        verbose_log!(self.config, success, "Solution found using ironshield-core!");
        verbose_kv!(self.config, "Solution Nonce", response.solution);
        verbose_kv!(self.config, "Time Taken", format!("{:?}", solve_duration));

        // Verify the solution using ironshield-core's verification.
        let is_valid = verify_ironshield_solution(&response);
        verbose_kv!(self.config, "Solution Verified", is_valid);

        if !is_valid {
            verbose_log!(self.config, error, "Solution verification failed - this should never happen");
            return Err(ErrorHandler::challenge_solving_error(
                "Solution verification failed - this should never happen"
            ));
        }

        Ok(response)
    }

    /// Performs the complete fetch-and-solve workflow.
    ///
    /// # Arguments
    /// * `endpoint` - The protected endpoint URL to access.
    ///
    /// # Returns
    /// * `ResultHandler<IronShieldChallengeResponse>` - The final solution.
    pub async fn fetch_and_solve(&self, endpoint: &str) -> ResultHandler<IronShieldChallengeResponse> {
        let total_start = Instant::now();

        // Step 1: Fetch the challenge.
        let challenge = self.fetch_challenge(endpoint).await?;

        // Step 2: Solve the challenge.
        let solution = self.solve_challenge(&challenge).await?;

        let total_duration = total_start.elapsed();
        verbose_log!(self.config, success, "Complete workflow finished in {:?}", total_duration);

        Ok(solution)
    }

    /// Submits a solution to verify it works with a protected endpoint.
    ///
    /// # Arguments
    /// * `solution` - The solved challenge response.
    /// * `target_url` - The protected endpoint to access.
    ///
    /// # Returns
    /// * `ResultHandler<String>` - The response from the protected endpoint.
    pub async fn submit_solution(
        &self,
        solution: &IronShieldChallengeResponse,
        target_url: &str,
    ) -> ResultHandler<String> {
        verbose_section!(self.config, "Solution Submission");
        verbose_log!(self.config, submit, "Submitting solution to: {}", target_url);

        let encoded_response = solution.to_base64url_header();
        verbose_kv!(self.config, "Encoded Response Length", encoded_response.len());

        let mut headers = HeaderMap::new();
        headers.insert("X-IronShield-Response", encoded_response.parse().unwrap());

        let response = self
            .http_client
            .get(target_url)
            .headers(headers)
            .send()
            .await
            .map_err(ErrorHandler::from_network_error)?;

        let status = response.status();
        let body = response.text().await
            .map_err(ErrorHandler::from_network_error)?;

        verbose_log!(self.config, receive, "Response status: {}", status);
        verbose_kv!(self.config, "Response Length", body.len());

        if status.is_success() {
            verbose_log!(self.config, success, "Solution successfully verified by protected endpoint");
            Ok(body)
        } else {
            verbose_log!(self.config, error, "Protected endpoint rejected solution: {}", status);
            Err(ErrorHandler::challenge_verification_error(format!(
                "Protected endpoint returned status {}: {}",
                status, body
            )))
        }
    }
}

