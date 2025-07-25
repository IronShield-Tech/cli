use ironshield::{
    IronShieldClient,
    solve_challenge,
    ClientConfig,
    IronShieldChallenge,
    IronShieldChallengeResponse,
    SolveConfig
};
use crate::display::{ProgressAnimation, format_number_with_commas};
use std::time::Instant;

/// CLI wrapper around the library's solve_challenge function that adds display logic
pub async fn solve_challenge_with_display(
    challenge: IronShieldChallenge,
    config: &ClientConfig,
    use_multithreaded: bool,
) -> Result<IronShieldChallengeResponse, ironshield_api::handler::error::ErrorHandler> {
    // Log configuration details
    crate::verbose_section!(config, "Challenge Solving");
    let solve_config = SolveConfig::new(config, use_multithreaded);
    crate::verbose_kv!(config, "Thread Count", solve_config.thread_count);
    crate::verbose_kv!(config, "Multithreaded", solve_config.use_multithreaded);
    crate::verbose_kv!(config, "Recommended Attempts", challenge.recommended_attempts);

    // Always show challenge difficulty info (both verbose and non-verbose modes)
    let difficulty: u64 = challenge.recommended_attempts / 2; // recommended_attempts = difficulty * 2
    println!("Received proof-of-work challenge with difficulty {}", format_number_with_commas(difficulty));

    // Start the progress animation (only in non-verbose mode)
    let animation = ProgressAnimation::new(config.verbose);
    let animation_handle = animation.start();

    let start_time = Instant::now();

    // Call the library's solve function
    let result = solve_challenge(challenge, config, use_multithreaded).await;

    // Stop the animation and clean up the line
    animation.stop(animation_handle).await;

    // Log timing and performance metrics
    match &result {
        Ok(solution) => {
            log_solution_performance(solution, start_time.elapsed(), &solve_config, config);
            println!("Challenge solved successfully!");
        },
        Err(e) => {
            crate::verbose_log!(
                config,
                error,
                "Challenge solving failed after {:?}: {}",
                start_time.elapsed(),
                e
            );
            // Error will be handled by the caller
        }
    }

    result
}

/// Log performance metrics for a solved challenge
fn log_solution_performance(
    solution: &IronShieldChallengeResponse,
    elapsed: std::time::Duration,
    solve_config: &SolveConfig,
    config: &ClientConfig,
) {
    let elapsed_millis: u64 = elapsed.as_millis() as u64;

    // Calculate estimated total attempts across all threads using thread-stride analysis
    let solution_nonce: u64 = solution.solution as u64;
    let estimated_attempts_per_thread: u64 = (solution_nonce / solve_config.thread_count as u64) + 1;
    let estimated_total_attempts: u64 = estimated_attempts_per_thread * solve_config.thread_count as u64;

    let hash_rate: u64 = if elapsed_millis > 0 {
        (estimated_total_attempts * 1000) / elapsed_millis
    } else {
        estimated_total_attempts  // If solved instantly, assume 1ms
    };

    crate::verbose_log!(
        config,
        timing,
        "Challenge solved in {:?} (~{} estimated total attempts, ~{} h/s)",
        elapsed,
        estimated_total_attempts,
        hash_rate
    );

    crate::verbose_log!(
        config,
        success,
        "Performance: {} threads achieved ~{} hashes/second (solution found at nonce {})",
        solve_config.thread_count,
        hash_rate,
        solution_nonce
    );
}

/// Handles the solve command - fetches and solves a challenge from the specified endpoint
pub async fn handle_solve(
    client: &IronShieldClient, 
    config: &ClientConfig,
    endpoint: &str, 
    single_threaded: bool
) -> color_eyre::Result<()> {
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

    // invert single_threaded flag to get use_multithreaded.
    let solution = solve_challenge_with_display(challenge, config, !single_threaded).await?;

    println!("Solution: {:?}", solution);

    std::process::exit(0);
} 