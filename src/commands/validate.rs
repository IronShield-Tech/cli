use ironshield::{
    IronShieldClient,
    ClientConfig,
};
use super::solve::solve_challenge_with_display;
use std::time::Instant;

/// Handles the validate command - fetches, solves, and validates a challenge from the specified endpoint
pub async fn handle_validate(
    client: &IronShieldClient, 
    config: &ClientConfig,
    endpoint: &str, 
    single_threaded: bool
) -> color_eyre::Result<()> {
    // Fetch the challenge
    crate::verbose_section!(config, "Challenge Fetching");
    crate::verbose_log!(config, network, "Requesting challenge for endpoint: {}", endpoint);

    let fetch_start = Instant::now();
    let challenge = client.fetch_challenge(endpoint).await?;

    crate::verbose_log!(
        config,
        timing,
        "Challenge fetch completed in {:?}",
        fetch_start.elapsed()
    );

    println!("Challenge fetched successfully!");

    crate::verbose_kv!(config, "Random Nonce", format!("{:?}", challenge.random_nonce));
    crate::verbose_kv!(config, "Difficulty", challenge.recommended_attempts / 2);
    crate::verbose_kv!(config, "Recommended Attempts", challenge.recommended_attempts);

    // Solve the challenge using our display wrapper
    let solution = solve_challenge_with_display(challenge, config, !single_threaded).await?;

    // Submit the solution for validation
    crate::verbose_section!(config, "Solution Submission");
    crate::verbose_log!(config, network, "Submitting solution...");

    let submit_start = Instant::now();
    let token = client.submit_solution(&solution).await?;

    crate::verbose_log!(
        config,
        timing,
        "Solution submission completed in {:?}",
        submit_start.elapsed()
    );

    println!("Challenge validated successfully!");
    
    crate::verbose_log!(config, success, "Token generated successfully!");
    crate::verbose_kv!(config, "Token Valid Until", token.valid_for);

    println!("Token: {token:?}");

    std::process::exit(0);
} 