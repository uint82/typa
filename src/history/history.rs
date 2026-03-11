use crate::app::App;
use crate::models::Mode;
use anyhow::Result;
use chrono::Utc;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
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
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    // old format was a json array. '[' at the start gives it away.
    // parsed and returned as-is; the next record_test() call will migrate it to jsonl.
    if trimmed.starts_with('[') {
        let records: Vec<TestRecord> = serde_json::from_str(trimmed)?;
        return Ok(records);
    }

    let mut records = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: TestRecord = serde_json::from_str(line)?;
        records.push(record);
    }
    Ok(records)
}




pub fn delete_record(index_newest_first: usize, total: usize) -> Result<()> {
    let Some(path) = history_path() else { return Ok(()); };
    if !path.exists() { return Ok(()); }

    let file_index = total - 1 - index_newest_first;

    let raw = fs::read_to_string(&path)?;
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if file_index >= lines.len() { return Ok(()); }

    let tmp_path = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp_path)?;
        for (i, line) in lines.iter().enumerate() {
            if i != file_index {
                writeln!(f, "{}", line)?;
            }
        }
        f.flush()?;
    }
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

pub fn clear_history() -> Result<()> {
    let Some(path) = history_path() else {
        return Ok(());
    };
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn record_test(app: &App, completed: bool) -> Result<()> {
    let duration_secs = app.test.start_time
        .map(|t| t.elapsed().as_secs_f64())
        .unwrap_or(0.0);

    // bail early. no point saving a test the user barely started.
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

    let record = TestRecord {
        timestamp,
        completed,
        mode: mode_str,
        mode_value,
        language:        app.config.word_data.name.clone(),
        use_punctuation: app.config.use_punctuation,
        use_numbers:     app.config.use_numbers,
        duration_secs,

        wpm:         completed.then(|| app.test.final_wpm),
        raw_wpm:     completed.then(|| app.test.final_raw_wpm),
        accuracy:    completed.then(|| app.test.final_accuracy),
        consistency: completed.then(|| app.test.final_consistency),

        correct_chars:        completed.then_some(correct_chars),
        incorrect_chars:      completed.then_some(incorrect_chars),
        extra_chars:          completed.then_some(extra_chars),
        missed_chars:         completed.then_some(missed_chars),
        correct_keystrokes:   completed.then_some(app.test.live_correct_keystrokes),
        incorrect_keystrokes: completed.then_some(app.test.live_incorrect_keystrokes),
        total_keystrokes:     completed.then_some(
            app.test.live_correct_keystrokes + app.test.live_incorrect_keystrokes
        ),

        quote_source,
        wpm_history:     completed.then(|| app.test.wpm_history.clone()),
        raw_wpm_history: completed.then(|| app.test.raw_wpm_history.clone()),
        errors_history:  completed.then(|| app.test.errors_history.clone()),
    };

    let Some(path) = history_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // one-time migration: old json array gets rewritten as jsonl before we append.
    // writes to a .tmp file first so a crash mid-write can't corrupt or destroy history.
    // rename() is atomic on every OS we care about; the old file survives any earlier failure.
    if path.exists() {
        let existing = fs::read_to_string(&path)?;
        if existing.trim_start().starts_with('[') {
            let old_records: Vec<TestRecord> = serde_json::from_str(existing.trim())?;
            let tmp_path = path.with_extension("tmp");
            {
                let mut f = fs::File::create(&tmp_path)?;
                for r in &old_records {
                    writeln!(f, "{}", serde_json::to_string(r)?)?;
                }
                f.flush()?;
            }
            fs::rename(&tmp_path, &path)?;
        }
    }

    // append only. O(1) no matter how long the history gets. that's the whole point of jsonl.
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{}", serde_json::to_string(&record)?)?;
    Ok(())
}
