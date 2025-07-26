use ironshield::{IronShieldClient, ClientConfig};
use std::time::Instant;

pub async fn handle_fetch(
    client: &IronShieldClient, 
    config: &ClientConfig,
    endpoint: &str
) -> color_eyre::Result<()> {
    crate::verbose_section!(config, "Challenge Fetching");
    crate::verbose_log!(config, network, "Requesting challenge for endpoint: {}", endpoint);

    let start_time = Instant::now();
    let challenge = client.fetch_challenge(endpoint).await?;

    crate::verbose_log!(
        config,
        timing,
        "Challenge fetch completed in {:?}",
        start_time.elapsed()
    );

    println!("Challenge fetched successfully!");
    println!("Recommended attempts: {}", challenge.recommended_attempts);

    crate::verbose_kv!(config, "Random Nonce", format!("{:?}", challenge.random_nonce));
    crate::verbose_kv!(config, "Difficulty", challenge.recommended_attempts / 2);
    crate::verbose_kv!(config, "Recommended Attempts", challenge.recommended_attempts);

    std::process::exit(0);
} 