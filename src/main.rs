mod config;
mod client;
mod util;
mod error;
mod constant;

use color_eyre::Result;
use crossterm::event::{
    Event,
    EventStream,
    KeyCode,
    KeyEventKind,
    KeyModifiers
};
use futures::{
    FutureExt,
    StreamExt
};
use ratatui::{
    DefaultTerminal,
    Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use clap::{
    Parser,
    Subcommand
};

use crate::client::{
    request::IronShieldClient,
    solve,
    validate
};
use crate::{
    config::ClientConfig,
    error::CliError,
};
use ironshield_types::{
    IronShieldChallenge,
    IronShieldChallengeResponse
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args: CliArgs = CliArgs::parse()?;

    let config: ClientConfig = ClientConfig::from_file(&args.config_path)?;

    let client: IronShieldClient = IronShieldClient::new(config.clone())?;

    // Execute the command based on CLI arguments.
    match args.command {
        Commands::Fetch { endpoint } => {
            let challenge = client.fetch_challenge(&endpoint).await?;
            println!("Challenge fetched successfully!");
            println!("Recommended attempts: {}", challenge.recommended_attempts);

            // Force clean exit to prevent hanging from aborted background threads
            std::process::exit(0);
        },
        Commands::Solve { endpoint, single_threaded } => {
            // First fetch the challenge
            let challenge: IronShieldChallenge = client.fetch_challenge(&endpoint).await?;
            println!("Challenge fetched successfully!");

            // Then solve it (invert single_threaded flag to get use_multithreaded)
            let solution: IronShieldChallengeResponse =
                solve::solve_challenge(challenge, &config, !single_threaded).await?;

            println!("Challenge solved successfully!");
            println!("Solution: {:?}", solution);

            // Force clean exit to prevent hanging from aborted background threads
            std::process::exit(0);
        },
        Commands::Validate { endpoint, single_threaded } => {
            let token =
                validate::validate_challenge(&client, &config, &endpoint, !single_threaded)
                    .await?;

            println!("Challenge validated successfully!");
            println!("Token: {:?}", token);

            // Force clean exit to prevent hanging from aborted background threads
            std::process::exit(0);
        }
    };
}

#[derive(Parser)]
#[command(
    name = "ironshield",
    about = "IronShield CLI - Fetch and solve proof-of-work challenges",
    version,
    long_about = "A command-line interface for interacting with IronShield proof-of-work \
                  challenge systems. Supports fetching challenges, solving them, and \
                  verifying solutions for protected endpoints."
)]
pub struct CliArgs {
    #[arg(
        short,
        long,
        default_value = "ironshield.toml",
        help = "Path to the configuration file."
    )]
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
        #[arg(
            short,
            long,
            help = "The protected endpoint URL to request a challenge for."
        )]
        endpoint: String,
    },
    Solve {
        #[arg(
            short,
            long,
            help = "The protected endpoint URL to solve a challenge for."
        )]
        endpoint: String,

        #[arg(
            short = 's',
            long = "single-threaded",
            help = "Use single-threaded solving instead of the default multithreaded approach."
        )]
        single_threaded: bool,
    },
    Validate {
        #[arg(
            short,
            long,
            help = "The protected endpoint URL to validate a challenge for."
        )]
        endpoint: String,

        #[arg(
            short = 's',
            long = "single-threaded",
            help = "Use single-threaded solving instead of the default multithreaded approach."
        )]
        single_threaded: bool,
    }
}

impl CliArgs {
    /// Parse command line arguments and return the structured CLI arguments.
    pub fn parse() -> Result<Self, CliError> {
        Ok(Parser::parse())
    }
}

#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running:      bool,
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