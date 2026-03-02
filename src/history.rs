use crate::app::App;
use crate::models::Mode;
use anyhow::Result;
use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestRecord {
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
    pub consistency: Option<f64>,
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

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(records)?;
    fs::write(&path, json)?;
    Ok(())
}



pub fn record_test(app: &App, completed: bool) -> Result<()> {
    let duration_secs = app.test.start_time
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    // don't save if the user quit before typing anything meaningful
    if duration_secs < 1.0 {
        return Ok(());
    }

    let (mode_str, mode_value) = match &app.config.mode {
        Mode::Time(t)  => ("time".to_string(),  t.to_string()),
        Mode::Words(w) => ("words".to_string(), w.to_string()),
        Mode::Quote(q) => {
            use crate::models::QuoteSelector;
            use crate::ui::utils::get_quote_length_category;
            let label = match q {
                QuoteSelector::Id(_) => get_quote_length_category(app.test.original_quote_length).to_string(),
                QuoteSelector::Category(len) => {
                    let s = format!("{:?}", len).to_lowercase();
                    if s == "all" {
                        get_quote_length_category(app.test.original_quote_length).to_string()
                    } else {
                        s
                    }
                }
            };
            ("quote".to_string(), label)
        }
    };

    let quote_source = if app.test.current_quote_source.is_empty() {
        None
    } else {
        Some(app.test.current_quote_source.clone())
    };

    let timestamp = Utc::now().to_rfc3339();

    let (correct_chars, incorrect_chars, extra_chars, missed_chars) = app.resolved_char_stats();

    let record = if completed {
        TestRecord {
            timestamp,
            completed: true,
            mode: mode_str,
            mode_value,
            language:        app.config.word_data.name.clone(),
            use_punctuation: app.config.use_punctuation,
            use_numbers:     app.config.use_numbers,
            duration_secs,

            wpm:         Some(app.test.final_wpm),
            raw_wpm:     Some(app.test.final_raw_wpm),
            accuracy:    Some(app.test.final_accuracy),
            consistency: Some(app.test.final_consistency),

            correct_chars:        Some(correct_chars),
            incorrect_chars:      Some(incorrect_chars),
            extra_chars:          Some(extra_chars),
            missed_chars:         Some(missed_chars),
            correct_keystrokes:   Some(app.test.live_correct_keystrokes),
            incorrect_keystrokes: Some(app.test.live_incorrect_keystrokes),
            total_keystrokes:     Some(app.test.live_correct_keystrokes + app.test.live_incorrect_keystrokes),

            quote_source,
            wpm_history:     Some(app.test.wpm_history.clone()),
            raw_wpm_history: Some(app.test.raw_wpm_history.clone()),
            errors_history:  Some(app.test.errors_history.clone()),
        }
    } else {
        TestRecord {
            timestamp,
            completed: false,
            mode: mode_str,
            mode_value,
            language:        app.config.word_data.name.clone(),
            use_punctuation: app.config.use_punctuation,
            use_numbers:     app.config.use_numbers,
            duration_secs,

            wpm:         None,
            raw_wpm:     None,
            accuracy:    None,
            consistency: None,

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
