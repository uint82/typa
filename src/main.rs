mod app;
mod config;
mod models;
mod ui;
mod history;
pub mod utils;
mod generator;
mod discord;

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

    /// Include numbers in the test
    #[arg(short, long, default_value_t = false, help_heading = "Flags")]
    numbers: bool,

    /// Include punctuation in the test
    #[arg(short, long, default_value_t = false, help_heading = "Flags")]
    punctuation: bool,

    /// Show interactive typing stats and history
    #[arg(long, default_value_t = false, help_heading = "Flags")]
    stats: bool,

    /// Delete all saved history (will prompt for confirmation)
    #[arg(long, default_value_t = false, help_heading = "Flags")]
    clear_history: bool,

    /// Print help
    #[arg(short, long, action = ArgAction::Help, help_heading = "Flags")]
    help: Option<bool>,

    /// Print version
    #[arg(short = 'V', long, action = ArgAction::Version, help_heading = "Flags")]
    version: Option<bool>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let app_config = AppConfig::load().unwrap_or_else(|e| {
        eprintln!(
            "Warning: Failed to load config, using defaults. Error: {}",
            e
        );
        AppConfig {
            theme: config::Theme::default(),
        }
    });

    if cli.clear_history {
        use std::io::{BufRead, Write};
        print!("  delete all history? this cannot be undone. [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        if input.trim().eq_ignore_ascii_case("y") {
            history::clear_history()?;
            println!("  history cleared.");
        } else {
            println!("  cancelled.");
        }
        return Ok(());
    }

    if cli.stats {
        let mut dp = crate::discord::DiscordPresence::new();
        if dp.connected {
            if let Ok(records) = history::load_history() {
                use crate::history::stats::compute_streaks;
                let completed: Vec<_> = records.iter().filter(|r| r.completed).collect();
                let best_wpm = completed.iter()
                    .filter_map(|r| r.wpm)
                    .fold(0.0_f64, f64::max);
                let total_tests = records.len();
                let (current_streak, _) = compute_streaks(&records);
                dp.set_stats(best_wpm, total_tests, current_streak);
            }
        }
        history::run(app_config.theme)?;
        return Ok(());
    }

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
    use std::time::Duration;

    let size = terminal.size()?;
    app.resize(size.width, size.height);

    const BLINK_PERIOD: Duration = Duration::from_millis(530);

    let mut last_blink_phase = u128::MAX;
    let mut last_timer_secs = u64::MAX;
    let mut needs_redraw = true;

    loop {
        app.check_time();

        let blink_phase = app.test.caret_epoch.elapsed().as_millis() / BLINK_PERIOD.as_millis();
        if blink_phase != last_blink_phase {
            last_blink_phase = blink_phase;
            needs_redraw = true;
        }

        if let Some(start) = app.test.start_time {
            let secs = start.elapsed().as_secs();
            if secs != last_timer_secs {
                last_timer_secs = secs;
                needs_redraw = true;
            }
        }

        if needs_redraw {
            terminal.draw(|f| ui::render(f, app))?;
            needs_redraw = false;
        }

        if event::poll(Duration::from_millis(100))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        needs_redraw = true;
                        match key.code {
                            KeyCode::Esc => app.quit(),
                            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.quit()
                            }
                            KeyCode::Tab => app.restart_test(),
                            KeyCode::Char('r') if app.test.state == models::AppState::Finished => app.retry_last_test(),
                            KeyCode::Char(c) => app.on_key(c),
                            KeyCode::Backspace => app.on_backspace(),
                            _ => { needs_redraw = false; }
                        }
                    }
                }
                Event::Mouse(_) => {
                    app.on_mouse();
                    needs_redraw = true;
                }
                Event::Resize(w, h) => {
                    app.resize(w, h);
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
