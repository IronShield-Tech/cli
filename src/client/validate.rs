use crate::{
    config::ClientConfig,
};
use crate::client::{
    client::IronShieldClient,
    solve,
};
use ironshield_api::handler::result::ResultHandler;
use ironshield_types::IronShieldToken;

/// Fetches a challenge, solves it, and submits the solution for validation.
///
/// This function orchestrates the entire validation flow:
/// 1. Fetches a new challenge from the API for the specified endpoint.
/// 2. Solves the challenge using either a multithreaded or single-threaded approach.
/// 3. Submits the solved challenge back to the API.
/// 4. Returns the received `IronShieldToken` upon successful validation.
///
/// # Arguments
///
/// * `client` - An instance of `IronShieldClient` to communicate with the API.
/// * `config` - The client configuration.
/// * `endpoint` - The protected endpoint URL to get a challenge for.
/// * `use_multithread` - A boolean indicating whether to use multithreaded solving.
///
/// # Returns
///
/// A `ResultHandler` containing the `IronShieldToken` if successful, or an error.
pub async fn validate_challenge(
    client: &IronShieldClient,
    config: &ClientConfig,
    endpoint: &str,
    use_multithread: bool,
) -> ResultHandler<IronShieldToken> {
    // 1. Fetch challenge
    crate::verbose_section!(config, "Fetching Challenge");
    let challenge = client.fetch_challenge(endpoint).await?;
    crate::verbose_log!(config, success, "Challenge fetched successfully!");

    // 2. Solve challenge
    crate::verbose_section!(config, "Solving Challenge");
    let solution = solve::solve_challenge(challenge, config, use_multithread).await?;
    crate::verbose_log!(config, success, "Challenge solved successfully!");

    // 3. Submit solution for validation
    crate::verbose_section!(config, "Submitting Solution");
    let token = client.submit_solution(&solution).await?;
    crate::verbose_log!(config, success, "Solution validated successfully!");

    Ok(token)
} 