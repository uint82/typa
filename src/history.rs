use crate::app::App;
use crate::models::Mode;
use anyhow::Result;
use chrono::Utc;
use crossterm::terminal as term;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestRecord {
    /// ISO 8601 UTC timestamp for date when test ended
    pub timestamp: String,
    pub completed: bool,
    pub mode: String,
    pub mode_value: String,
    pub language: String,
    pub use_punctuation: bool,
    pub use_numbers: bool,

    pub wpm: Option<f64>,
    pub raw_wpm: Option<f64>,
    pub accuracy: Option<f64>,
    pub duration_secs: f64,

    pub correct_chars: Option<usize>,
    pub incorrect_chars: Option<usize>,
    pub extra_chars: Option<usize>,
    pub missed_chars: Option<usize>,
    pub correct_keystrokes: Option<usize>,
    pub incorrect_keystrokes: Option<usize>,
    pub total_keystrokes: Option<usize>,

    pub quote_source: Option<String>,

    pub wpm_history: Option<Vec<(f64, f64)>>,
    pub raw_wpm_history: Option<Vec<(f64, f64)>>,
    pub errors_history: Option<Vec<(f64, f64)>>,
}


fn history_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "typa").map(|dirs| dirs.data_local_dir().join("history.json"))
}

pub fn load_history() -> Result<Vec<TestRecord>> {
    let Some(path) = history_path() else {
        return Ok(Vec::new());
    };

    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&path)?;
    let records: Vec<TestRecord> = serde_json::from_str(&raw)?;
    Ok(records)
}

fn save_history(records: &[TestRecord]) -> Result<()> {
    let Some(path) = history_path() else {
        return Ok(());
    };

    // create the data directory if it doesn't exist yet
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(records)?;
    fs::write(&path, json)?;
    Ok(())
}


pub fn show_history() -> Result<()> {
    let records = load_history()?;

    if records.is_empty() {
        println!("\n  No history yet. Complete a test to start tracking your progress.\n");
        return Ok(());
    }

    let term_width = term::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
        .max(40);

    const W_NUM:  usize = 5;
    const W_DATE: usize = 12;
    const W_MODE: usize = 12;
    const W_LANG: usize = 12;
    const W_WPM:  usize = 7;
    const W_RAW:  usize = 7;
    const W_ACC:  usize = 9;
    const W_TIME: usize = 8;
    const W_DONE: usize = 5;

    let base_width = 1 + W_NUM + W_DATE + W_WPM + W_ACC + W_DONE;
    let show_mode = term_width >= base_width + W_MODE;
    let show_lang = term_width >= base_width + W_MODE + W_LANG;
    let show_raw  = term_width >= base_width + W_MODE + W_LANG + W_RAW;
    let show_time = term_width >= base_width + W_MODE + W_LANG + W_RAW + W_TIME;

    let total_width = 1
        + W_NUM + W_DATE + W_WPM + W_ACC + W_DONE
        + if show_mode { W_MODE } else { 0 }
        + if show_lang { W_LANG } else { 0 }
        + if show_raw  { W_RAW  } else { 0 }
        + if show_time { W_TIME } else { 0 };

    let divider = "-".repeat(total_width);

    let completed: Vec<&TestRecord> = records.iter().filter(|r| r.completed).collect();
    let total = records.len();
    let done  = completed.len();

    println!();
    if !completed.is_empty() {
        let avg_wpm  = completed.iter().filter_map(|r| r.wpm).sum::<f64>() / done as f64;
        let avg_acc  = completed.iter().filter_map(|r| r.accuracy).sum::<f64>() / done as f64;
        let best_wpm = completed.iter().filter_map(|r| r.wpm).fold(0.0_f64, f64::max);

        println!(
            "  {} tests  |  {} completed  |  avg wpm {:.0}  |  best wpm {:.0}  |  avg acc {:.2}%",
            total, done, avg_wpm, best_wpm, avg_acc
        );
    } else {
        println!("  {} tests total  |  {} completed", total, done);
    }
    println!();

    print!(" {:<nw$}{:<dw$}", "#", "date", nw = W_NUM, dw = W_DATE);
    if show_mode { print!("{:<mw$}", "mode",     mw = W_MODE); }
    if show_lang { print!("{:<lw$}", "language", lw = W_LANG); }
    print!("{:<ww$}", "wpm", ww = W_WPM);
    if show_raw  { print!("{:<rw$}", "raw",  rw = W_RAW);  }
    print!("{:<aw$}", "acc", aw = W_ACC);
    if show_time { print!("{:<tw$}", "time", tw = W_TIME); }
    println!("done");
    println!(" {}", divider);

    for (i, r) in records.iter().rev().enumerate() {
        let date = r.timestamp.get(..10).unwrap_or(&r.timestamp).to_string();
        let mode = format!("{} {}", r.mode, r.mode_value);
        let wpm  = r.wpm.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".to_string());
        let raw  = r.raw_wpm.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".to_string());
        let acc  = r.accuracy.map(|v| format!("{:.2}%", v)).unwrap_or_else(|| "-".to_string());
        let time = format!("{:.1}s", r.duration_secs);
        let done = if r.completed { "Y" } else { "N" };

        print!(" {:<nw$}{:<dw$}", i + 1, date, nw = W_NUM, dw = W_DATE);
        if show_mode { print!("{:<mw$}", mode,        mw = W_MODE); }
        if show_lang { print!("{:<lw$}", r.language,  lw = W_LANG); }
        print!("{:<ww$}", wpm, ww = W_WPM);
        if show_raw  { print!("{:<rw$}", raw,  rw = W_RAW);  }
        print!("{:<aw$}", acc, aw = W_ACC);
        if show_time { print!("{:<tw$}", time, tw = W_TIME); }
        println!("{}", done);
    }

    println!(" {}", divider);
    println!();

    Ok(())
}


pub fn record_test(app: &App, completed: bool) -> Result<()> {
    let duration_secs = app
        .start_time
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    // don't save if the user quit before typing anything meaningful
    if duration_secs < 1.0 {
        return Ok(());
    }

    let (mode_str, mode_value) = match &app.mode {
        Mode::Time(t)  => ("time".to_string(),  t.to_string()),
        Mode::Words(w) => ("words".to_string(), w.to_string()),
        Mode::Quote(q) => {
            use crate::models::QuoteSelector;
            use crate::ui::utils::get_quote_length_category;
            let label = match q {
                QuoteSelector::Id(_) => get_quote_length_category(app.original_quote_length).to_string(),
                QuoteSelector::Category(len) => {
                    let s = format!("{:?}", len).to_lowercase();
                    if s == "all" {
                        get_quote_length_category(app.original_quote_length).to_string()
                    } else {
                        s
                    }
                }
            };
            ("quote".to_string(), label)
        }
    };

    let quote_source = if app.current_quote_source.is_empty() {
        None
    } else {
        Some(app.current_quote_source.clone())
    };

    let timestamp = Utc::now().to_rfc3339();

    let (correct_chars, incorrect_chars, extra_chars, missed_chars) = app.resolved_char_stats();

    let record = if completed {
        TestRecord {
            timestamp,
            completed: true,
            mode: mode_str,
            mode_value,
            language: app.word_data.name.clone(),
            use_punctuation: app.use_punctuation,
            use_numbers: app.use_numbers,
            duration_secs,

            wpm:      Some(app.final_wpm),
            raw_wpm:  Some(app.final_raw_wpm),
            accuracy: Some(app.final_accuracy),

            correct_chars:        Some(correct_chars),
            incorrect_chars:      Some(incorrect_chars),
            extra_chars:          Some(extra_chars),
            missed_chars:         Some(missed_chars),
            correct_keystrokes:   Some(app.live_correct_keystrokes),
            incorrect_keystrokes: Some(app.live_incorrect_keystrokes),
            total_keystrokes:     Some(app.live_correct_keystrokes + app.live_incorrect_keystrokes),

            quote_source,
            wpm_history:     Some(app.wpm_history.clone()),
            raw_wpm_history: Some(app.raw_wpm_history.clone()),
            errors_history:  Some(app.errors_history.clone()),
        }
    } else {
        TestRecord {
            timestamp,
            completed: false,
            mode: mode_str,
            mode_value,
            language: app.word_data.name.clone(),
            use_punctuation: app.use_punctuation,
            use_numbers: app.use_numbers,
            duration_secs,

            wpm:      None,
            raw_wpm:  None,
            accuracy: None,

            correct_chars:        None,
            incorrect_chars:      None,
            extra_chars:          None,
            missed_chars:         None,
            correct_keystrokes:   None,
            incorrect_keystrokes: None,
            total_keystrokes:     None,

            quote_source,
            wpm_history:     None,
            raw_wpm_history: None,
            errors_history:  None,
        }
    };

    let mut records = load_history().unwrap_or_default();
    records.push(record);
    save_history(&records)?;
    Ok(())
}
