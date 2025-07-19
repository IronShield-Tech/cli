use serde_json::Value;
use ironshield_api::handler::{
    error::ErrorHandler,
    result::ResultHandler
};
use ironshield_types::IronShieldChallenge;

/// Represents a structured IronShield API response.
///
/// * `status`: HTTP status code from the
///             API response.
/// * `message: Human-readable message
///             from the API.
/// * `data`:   Raw JSON data containing
///             the full response payload.
pub struct ApiResponse {
    pub status:  u16,
    pub message: String,
    pub data:    Value
}

impl ApiResponse {
    /// Parses a raw JSON response into a structured `ApiResponse`.
    ///
    /// # Arguments
    /// * `response`: The raw JSON value from the API response.
    ///
    /// # Returns
    /// * `ResultHandler<Self>`: Parsed response or an error.
    ///
    /// # Example
    /// ```
    /// let json_response = serde_json::json!({
    ///     "status": 200,
    ///     "message": "Success",
    ///     "challenge": { /* challenge data */ }
    /// });
    /// let api_response = ApiResponse::from_json(json_response)?;
    /// ```
    pub fn from_json(response: Value) -> ResultHandler<Self> {
        let status = response.get("status")
            .and_then(|s| s.as_u64())
            .unwrap_or(0) as u16;

        let message = response.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("No message")
            .to_string();

        Ok(Self {
            status,
            message,
            data: response,
        })
    }

    /// # Returns
    /// * `bool`: `true` if the status code is 200 (OK),
    ///           `false` otherwise.
    pub fn is_success(&self) -> bool {
        self.status == 200
    }

    /// Extracts and deserializes challenge data from the
    /// API response.
    ///
    /// # Returns
    /// `ResultHandler<IronShieldChallenge>`: A parsed challenge
    ///                                       or an error if the
    ///                                       response indicates
    ///                                       failure or the
    ///                                       challenge data is
    ///                                       missing/invalid.
    pub fn extract_challenge(&self) -> ResultHandler<IronShieldChallenge> {
        if !self.is_success() {
            return Err(ErrorHandler::ProcessingError(format!(
                "API error ({}): {}", self.status, self.message
            )));
        }

        let challenge_data = self.data.get("challenge")
            .ok_or_else(|| ErrorHandler::ProcessingError(
                "No challenge field in API response".to_string()
            ))?;

        serde_json::from_value(challenge_data.clone())
            .map_err(|e| ErrorHandler::ProcessingError(format!(
                "Failed to deserialize challenge: {}", e
            )))
    }
}