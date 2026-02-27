mod app;
mod config;
mod models;
mod ui;
mod history;
pub mod utils;
mod generator;

use anyhow::Result;
use app::App;
use models::{Mode, QuoteLength, QuoteSelector};
use clap::builder::RangedU64ValueParser;
use clap::{ArgAction, ArgGroup, Parser};
use config::AppConfig;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[derive(Parser, Debug)]
#[command(name = "typa")]
#[command(version)]
#[command(about = "A rusty terminal typing test", long_about = None)]
// disable the default flags so i can customize them manually below
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(help_template = "\
{name} {version}
{about-section}
{usage-heading} {usage}

{all-args}
")]
#[command(group(
    ArgGroup::new("mode")
        .required(false)
        .args(&["time", "words", "quote"])
))]
struct Cli {
    // these stay under the default "Options" heading
    /// Time mode: Custom duration in seconds (e.g. 15, 60, 120, 3600)
    #[arg(short, long, value_parser = RangedU64ValueParser::<u64>::new().range(1..))]
    time: Option<u64>,

    /// Words mode: Word count (1 to 10000)
    #[arg(short, long, value_parser = RangedU64ValueParser::<u64>::new().range(1..=10000))]
    words: Option<u64>,

    /// Quote mode: "short", "medium", "long", "very_long", "all", or a specific ID (e.g. 25)
    #[arg(short, long)]
    quote: Option<String>,

    /// Language: Filename to use (e.g. "english", "indonesian")
    #[arg(short, long, default_value = "english")]
    language: String,

    // explicitly move these to a "Flags" heading
    /// Include numbers in the test
    #[arg(short, long, default_value_t = false, help_heading = "Flags")]
    numbers: bool,

    /// Include punctuation in the test
    #[arg(short, long, default_value_t = false, help_heading = "Flags")]
    punctuation: bool,

    /// Show typing test history
    #[arg(long, default_value_t = false, help_heading = "Flags")]
    history: bool,

    /// Print help
    #[arg(short, long, action = ArgAction::Help, help_heading = "Flags")]
    help: Option<bool>,

    /// Print version
    #[arg(short = 'V', long, action = ArgAction::Version, help_heading = "Flags")]
    version: Option<bool>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.history {
        history::show_history()?;
        return Ok(());
    }

    let app_config = AppConfig::load().unwrap_or_else(|e| {
        eprintln!(
            "Warning: Failed to load config, using defaults. Error: {}",
            e
        );
        // return a default instance if file loading completely crashes
        // (though load() handles missing files gracefully, this catches format errors)
        AppConfig {
            theme: config::Theme::default(),
        }
    });

    let initial_mode = if let Some(t) = cli.time {
        Mode::Time(t)
    } else if let Some(w) = cli.words {
        let count = w as usize;
        Mode::Words(count)
    } else if let Some(q_str) = cli.quote {
        if let Ok(id) = q_str.parse::<usize>() {
            Mode::Quote(QuoteSelector::Id(id))
        } else {
            let category = match q_str.to_lowercase().as_str() {
                "short" => QuoteLength::Short,
                "medium" => QuoteLength::Medium,
                "long" => QuoteLength::Long,
                "very_long" | "verylong" => QuoteLength::VeryLong,
                _ => QuoteLength::All,
            };
            Mode::Quote(QuoteSelector::Category(category))
        }
    } else {
        Mode::Time(60)
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(
        initial_mode,
        cli.language,
        cli.numbers,
        cli.punctuation,
        app_config.theme,
    )?;

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let size = terminal.size()?;
    app.resize(size.width, size.height);

    loop {
        terminal.draw(|f| ui::render(f, app))?;
        app.check_time();

        if event::poll(std::time::Duration::from_millis(16))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc => app.quit(),
                            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.quit()
                            }
                            KeyCode::Tab => app.restart_test(),
                            KeyCode::Char(c) => app.on_key(c),
                            KeyCode::Backspace => app.on_backspace(),
                            _ => {}
                        }
                    }
                }
                Event::Mouse(_) => {
                    app.on_mouse();
                }
                Event::Resize(w, h) => {
                    app.resize(w, h);
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
