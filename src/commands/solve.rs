use ironshield::{
    IronShieldClient,
    solve_challenge,
    ClientConfig,
    IronShieldChallenge,
    IronShieldChallengeResponse
};
use crate::display::{ProgressAnimation, format_number_with_commas};

/// CLI wrapper around the library's solve_challenge function that adds display logic
pub async fn solve_challenge_with_display(
    challenge: IronShieldChallenge,
    config: &ClientConfig,
    use_multithreaded: bool,
) -> Result<IronShieldChallengeResponse, ironshield_api::handler::error::ErrorHandler> {
    // Always show challenge difficulty info (both verbose and non-verbose modes)
    let difficulty: u64 = challenge.recommended_attempts / 2; // recommended_attempts = difficulty * 2
    println!("Received proof-of-work challenge with difficulty {}", format_number_with_commas(difficulty));

    // Start the progress animation (only in non-verbose mode)
    let animation = ProgressAnimation::new(config.verbose);
    let animation_handle = animation.start();

    // Call the library's solve function
    let result = solve_challenge(challenge, config, use_multithreaded).await;

    // Stop the animation and clean up the line
    animation.stop(animation_handle).await;

    // Print success message if the challenge was solved
    match &result {
        Ok(_) => {
            println!("Challenge solved successfully!");
        },
        Err(_) => {
            // Error will be handled by the caller
        }
    }

    result
}

/// Handles the solve command - fetches and solves a challenge from the specified endpoint
pub async fn handle_solve(
    client: &IronShieldClient, 
    config: &ClientConfig,
    endpoint: &str, 
    single_threaded: bool
) -> color_eyre::Result<()> {
    let challenge = client.fetch_challenge(endpoint).await?;
    println!("Challenge fetched successfully!");

    // invert single_threaded flag to get use_multithreaded.
    let solution = solve_challenge_with_display(challenge, config, !single_threaded).await?;

    println!("Solution: {:?}", solution);

    std::process::exit(0);
} 