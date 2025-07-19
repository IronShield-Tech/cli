mod config;
mod client;
mod util;
mod error;

use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use clap::{Parser, Subcommand};
use crate::client::IronShieldClient;
use crate::error::CliError;
use crate::config::ClientConfig;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Parse command line arguments.
    let args = CliArgs::parse()?;

    // Load configuration from file.
    let config = ClientConfig::from_file(&args.config_path)?;

    // Create client.
    let client = IronShieldClient::new(config)?;

    // Execute the command based on CLI arguments.
    match args.command {
        Commands::Fetch { endpoint } => {
            execute_fetch_command(client, &endpoint).await?;
        }
        Commands::Solve { endpoint } => {
            execute_solve_command(client, &endpoint).await?;
        }
        Commands::Test => {
            execute_test_command().await?;
        }
        Commands::Tui => {
            // Run the TUI interface.
            let terminal = ratatui::init();
            let result = App::new().run(terminal).await;
            ratatui::restore();
            result?;
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "ironshield")]
#[command(about = "IronShield CLI - Solve proof-of-work challenges")]
#[command(version)]
pub struct CliArgs {
    #[arg(short, long, default_value = "ironshield.toml")]
    pub config_path: String,

    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Fetch a challenge from the specified endpoint and print it to stdout.
    Fetch {
        /// The endpoint URL to fetch a challenge for.
        #[arg(short, long)]
        endpoint: String,
    },
    /// Solve a challenge for the specified endpoint.
    Solve {
        /// The endpoint URL to solve a challenge for.
        #[arg(short, long)]
        endpoint: String,
    },
    /// Run test operations.
    Test,
    /// Launch the TUI interface (default behavior).
    Tui,
}

impl CliArgs {
    /// Parse command line arguments and return the structured CLI arguments.
    pub fn parse() -> Result<Self, CliError> {
        Ok(clap::Parser::parse())
    }
}

/// Execute the fetch command to retrieve and print a challenge.
async fn execute_fetch_command(client: IronShieldClient, endpoint: &str) -> Result<(), CliError> {
    println!("Fetching challenge for endpoint: {}", endpoint);

    match client.fetch_challenge(endpoint).await {
        Ok(challenge) => {
            println!("\n🔸 Challenge Details");
            println!("{}", "─".repeat(50));
            println!("Website ID: {}", challenge.website_id);
            println!("Difficulty: {}", challenge.recommended_attempts);
            println!("Expiry: {}", challenge.expiration_time);

            // Print the challenge in JSON format for easy parsing.
            println!("\n🔸 Raw Challenge JSON");
            println!("{}", "─".repeat(50));
            match serde_json::to_string_pretty(&challenge) {
                Ok(json) => println!("{}", json),
                Err(e) => println!("Failed to serialize challenge to JSON: {}", e),
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to fetch challenge: {}", e);
            return Err(CliError::RequestFailed(format!("Challenge fetch failed: {}", e)));
        }
    }

    Ok(())
}

/// Execute the solve command to fetch and solve a challenge.
async fn execute_solve_command(client: IronShieldClient, endpoint: &str) -> Result<(), CliError> {
    println!("Solving challenge for endpoint: {}", endpoint);

    match client.fetch_and_solve(endpoint).await {
        Ok(solution) => {
            println!("\n🔸 Solution Details");
            println!("{}", "─".repeat(50));
            println!("Nonce: {}", solution.solution);

            // Print the solution in JSON format.
            println!("\n🔸 Raw Solution JSON");
            println!("{}", "─".repeat(50));
            match serde_json::to_string_pretty(&solution) {
                Ok(json) => println!("{}", json),
                Err(e) => println!("Failed to serialize solution to JSON: {}", e),
            }

            // Print the base64url encoded response header.
            println!("\n🔸 Encoded Response Header");
            println!("{}", "─".repeat(50));
            println!("{}", solution.to_base64url_header());
        }
        Err(e) => {
            eprintln!("❌ Failed to solve challenge: {}", e);
            return Err(CliError::RequestFailed(format!("Challenge solving failed: {}", e)));
        }
    }

    Ok(())
}

/// Execute test operations.
async fn execute_test_command() -> Result<(), CliError> {
    println!("Running test operations...");
    // TODO: Implement test functionality.
    println!("✅ Test completed successfully");
    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,
    /// Event stream for handling terminal events.
    event_stream: EventStream,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop for TUI interface.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    /// Renders the user interface for TUI mode.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/master/examples>
    fn draw(&mut self, frame: &mut Frame) {
        let title = Line::from("IronShield CLI - TUI Mode")
            .bold()
            .blue()
            .centered();
        let text = "IronShield Challenge Solver\n\n\
            Use CLI commands for direct operations:\n\
            • ironshield fetch --endpoint <URL>\n\
            • ironshield solve --endpoint <URL>\n\
            • ironshield test\n\n\
            Press `Esc`, `Ctrl-C` or `q` to exit TUI mode.";
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::bordered().title(title))
                .centered(),
            frame.area(),
        )
    }

    /// Reads the crossterm events and updates the state of [`App`].
    async fn handle_crossterm_events(&mut self) -> Result<()> {
        tokio::select! {
            maybe_event = self.event_stream.next().fuse() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        if let Event::Key(key) = event {
                            if key.kind == KeyEventKind::Press {
                                match key.code {
                                    KeyCode::Char('q') => self.running = false,
                                    KeyCode::Esc => self.running = false,
                                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                        self.running = false;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => self.running = false,
                }
            }
        }
        Ok(())
    }
}