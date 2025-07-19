mod config;
mod client;
mod util;
mod error;
mod response;
mod http_builder;
mod constant;

use crate::client::IronShieldClient;
use crate::error::CliError;
use crate::config::ClientConfig;

use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use clap::{Parser, Subcommand};

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
        Commands::Fetch { endpoint } => client.fetch_challenge(&endpoint).await?,
    };

    Ok(())
}

#[derive(Parser)]
#[command(name = "ironshield", about = "IronShield CLI - Solve proof-of-work challenges", version)]
pub struct CliArgs {
    #[arg(short, long, default_value = "ironshield.toml")]
    pub config_path: String,

    #[command(subcommand)]
    pub command: Commands,
}
#[derive(Subcommand)]
pub enum Commands {
    // Descriptions for CLI arguments are
    // denoted by adding a triple '/' (///)
    // above the enum variant.
    //
    // Example:
    //
    // enum Example {
    //     /// Command does this and that.
    //     Command { /* whatever it's fetching */ }
    // }
    /// Fetches an IronShield request as an object.
    Fetch {
        #[arg(short, long)]
        endpoint: String,
    },
//    /// Solve a challenge for the specified endpoint.
//    Solve {
//        /// The endpoint URL to solve a challenge for.
//        #[arg(short, long)]
//        endpoint: String,
//    },
//    /// Run test operations.
//    Test,
//    /// Launch the TUI interface (default behavior).
//    Tui,
}

impl CliArgs {
    /// Parse command line arguments and return the structured CLI arguments.
    pub fn parse() -> Result<Self, CliError> {
        Ok(Parser::parse())
    }
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