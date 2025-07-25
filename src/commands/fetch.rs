use ironshield::IronShieldClient;

/// Handles the fetch command - fetches a challenge from the specified endpoint
pub async fn handle_fetch(client: &IronShieldClient, endpoint: &str) -> color_eyre::Result<()> {
    let challenge = client.fetch_challenge(endpoint).await?;
    println!("Challenge fetched successfully!");
    println!("Recommended attempts: {}", challenge.recommended_attempts);

    std::process::exit(0);
} 