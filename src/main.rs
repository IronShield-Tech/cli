mod config;
mod util;
mod error;
mod constant;
mod display;
mod commands;

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

use ironshield::{
    IronShieldClient,
    ClientConfig,
};
use crate::error::CliError;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args: CliArgs = CliArgs::parse()?;

    let (config_path, verbose) = match &args.command {
        Commands::Fetch    { config_path, verbose, .. } => (config_path, *verbose),
        Commands::Solve    { config_path, verbose, .. } => (config_path, *verbose),
        Commands::Validate { config_path, verbose, .. } => (config_path, *verbose),
    };

    let mut config: ClientConfig = match config_path {
        Some(path) => ClientConfig::from_file(&path)?,
        None => {
            println!("No config file specified, using default configuration.");
            ClientConfig::default()
        }
    };

    if verbose || args.verbose {
        config.verbose = true
    };

    let client: IronShieldClient = IronShieldClient::new(config.clone())?;

    match args.command {
        Commands::Fetch { endpoint, .. } => {
            commands::fetch::handle_fetch(&client, &endpoint).await?;
        },
        Commands::Solve { endpoint, single_threaded, .. } => {
            commands::solve::handle_solve(&client, &config, &endpoint, single_threaded).await?;
        },
        Commands::Validate { endpoint, single_threaded, .. } => {
            commands::validate::handle_validate(&client, &config, &endpoint, single_threaded).await?;
        }
    }

    Ok(())
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
        help = "Enable verbose output (overrides config file setting)."
    )]
    pub verbose: bool,
    #[arg(
        short,
        long,
//      default_value = "ironshield.toml",
        help = "Path to the configuration file."
    )]
    pub config_path: Option<String>,

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
        /// The protected endpoint URL to request from.
        endpoint: String,

        #[arg(
            short,
            long,
            help = "Enable verbose output (overrides config file setting)."
        )]
        verbose: bool,

        #[arg(
            short,
            long,
            help = "Path to the configuration file."
        )]
        config_path: Option<String>,
    },
    Solve {
        /// The protected endpoint URL to solve for.
        endpoint: String,

        #[arg(
            short = 's',
            long = "single-threaded",
            help = "Use single-threaded solving instead of the default multithreaded approach."
        )]
        single_threaded: bool,
        #[arg(
            short,
            long,
            help = "Enable verbose output (overrides config file setting)."
        )]
        verbose: bool,
        #[arg(
            short,
            long,
            help = "Path to the configuration file."
        )]
        config_path: Option<String>,
    },
    Validate {
        /// The protected endpoint URL to validate a challenge with.
        endpoint: String,

        #[arg(
            short = 's',
            long = "single-threaded",
            help = "Use single-threaded solving instead of the default multithreaded approach."
        )]
        single_threaded: bool,
        #[arg(
            short,
            long,
            help = "Enable verbose output (overrides config file setting)."
        )]
        verbose: bool,
        #[arg(
            short,
            long,
            help = "Path to the configuration file."
        )]
        config_path: Option<String>,
    }
}

impl CliArgs {
    pub fn parse() -> Result<Self, CliError> {
        Ok(Parser::parse())
    }
}

#[derive(Debug, Default)]
pub struct App {
    running:      bool,
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