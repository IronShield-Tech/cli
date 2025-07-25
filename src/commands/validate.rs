use ironshield::{
    IronShieldClient,
    ClientConfig,
};
use super::solve::solve_challenge_with_display;

/// Handles the validate command - fetches, solves, and validates a challenge from the specified endpoint
pub async fn handle_validate(
    client: &IronShieldClient, 
    config: &ClientConfig,
    endpoint: &str, 
    single_threaded: bool
) -> color_eyre::Result<()> {
    // Fetch the challenge
    let challenge = client.fetch_challenge(endpoint).await?;
    println!("Challenge fetched successfully!");

    // Solve the challenge using our display wrapper
    let solution = solve_challenge_with_display(challenge, config, !single_threaded).await?;

    // Submit the solution for validation
    let token = client.submit_solution(&solution).await?;

    println!("Challenge validated successfully!");
    println!("Token: {:?}", token);

    std::process::exit(0);
} 