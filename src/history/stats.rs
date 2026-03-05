use super::history::TestRecord;
use chrono::{DateTime, Local, NaiveDate};
use std::collections::HashMap;

pub(crate) fn format_duration(total_secs: u64) -> String {
    if total_secs >= 3600 {
        format!("{}h {}m", total_secs / 3600, (total_secs % 3600) / 60)
    } else if total_secs >= 60 {
        format!("{}m {}s", total_secs / 60, total_secs % 60)
    } else {
        format!("{}s", total_secs)
    }
}

pub(crate) fn local_datetime(ts: &str) -> (String, String) {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        let local: DateTime<Local> = dt.with_timezone(&Local);
        (
            local.format("%Y-%m-%d").to_string(),
            local.format("%H:%M:%S").to_string(),
        )
    } else {
        (
            ts.get(..10).unwrap_or(ts).to_string(),
            ts.get(11..19).unwrap_or("").to_string(),
        )
    }
}

pub(crate) fn compute_streaks(records: &[TestRecord]) -> (usize, usize) {
    let mut dates: Vec<NaiveDate> = records.iter()
        .filter(|r| r.completed)
        .filter_map(|r| {
            DateTime::parse_from_rfc3339(&r.timestamp).ok()
                .map(|dt| dt.with_timezone(&Local).date_naive())
        })
        .collect();
    dates.sort();
    dates.dedup();

    if dates.is_empty() { return (0, 0); }

    let mut best        = 1usize;
    let mut current_run = 1usize;

    for i in 1..dates.len() {
        if (dates[i] - dates[i - 1]).num_days() == 1 {
            current_run += 1;
            if current_run > best { best = current_run; }
        } else {
            current_run = 1;
        }
    }

    let today     = Local::now().date_naive();
    let last_date = *dates.last().unwrap(); // safe: non-empty checked above
    let current   = if (today - last_date).num_days() <= 1 { current_run } else { 0 };

    (current, best)
}

pub(crate) struct StatSection {
    pub(crate) title:      String,
    pub(crate) col_header: Option<String>,
    pub(crate) rows:       Vec<(String, String)>,
}

/// built once on load and never touched again. it's not live.
pub(crate) fn build_stat_sections(records: &[TestRecord]) -> Vec<StatSection> {
    let completed: Vec<&TestRecord> = records.iter().filter(|r| r.completed).collect();
    let total      = records.len();
    let done       = completed.len();
    let incomplete = total - done;

    let lifetime_ks: usize = records.iter().filter_map(|r| r.total_keystrokes).sum();
    let total_secs: u64    = records.iter().map(|r| r.duration_secs as u64).sum();

    let mut sections: Vec<StatSection> = Vec::new();

    sections.push(StatSection {
        title: "overview".into(),
        col_header: None,
        rows: vec![
            ("tests".into(),        total.to_string()),
            ("completed".into(),    done.to_string()),
            ("incomplete".into(),   incomplete.to_string()),
            ("total time".into(),   format_duration(total_secs)),
            ("lifetime keys".into(), lifetime_ks.to_string()),
        ],
    });

    if completed.is_empty() {
        return sections;
    }

    let wpm_vals: Vec<f64> = completed.iter().filter_map(|r| r.wpm).collect();
    let raw_vals: Vec<f64> = completed.iter().filter_map(|r| r.raw_wpm).collect();
    let acc_vals: Vec<f64> = completed.iter().filter_map(|r| r.accuracy).collect();

    let (wpm_sum, best_wpm) = wpm_vals.iter()
        .fold((0.0_f64, 0.0_f64), |(s, b), &w| (s + w, b.max(w)));
    let avg_wpm = wpm_sum / wpm_vals.len().max(1) as f64;
    let avg_raw = raw_vals.iter().sum::<f64>() / raw_vals.len().max(1) as f64;
    let avg_acc = acc_vals.iter().sum::<f64>() / acc_vals.len().max(1) as f64;

    let mut perf_rows = vec![
        ("avg wpm".into(),  format!("{:.0}", avg_wpm)),
        ("best wpm".into(), format!("{:.0}", best_wpm)),
        ("avg raw".into(),  format!("{:.0}", avg_raw)),
        ("avg acc".into(),  format!("{:.1}%", avg_acc)),
    ];

    let con_vals: Vec<f64> = completed.iter().filter_map(|r| r.consistency).collect();
    if !con_vals.is_empty() {
        let avg_con = con_vals.iter().sum::<f64>() / con_vals.len() as f64;
        perf_rows.push(("avg con".into(), format!("{:.0}%", avg_con)));
    }

    sections.push(StatSection { title: "performance".into(), col_header: None, rows: perf_rows });

    let (current_streak, best_streak) = compute_streaks(records);
    if best_streak > 0 {
        sections.push(StatSection {
            title: "streaks".into(),
            col_header: None,
            rows: vec![
                ("current".into(), format!("{} days", current_streak)),
                ("best".into(),    format!("{} days", best_streak)),
            ],
        });
    }

    let mut mode_groups: HashMap<String, Vec<f64>> = HashMap::new();
    for r in &completed {
        if let Some(w) = r.wpm {
            mode_groups.entry(mode_bucket(&r.mode, &r.mode_value)).or_default().push(w);
        }
    }

    if !mode_groups.is_empty() {
        let mut mode_keys: Vec<&String> = mode_groups.keys().collect();
        mode_keys.sort();

        let mut rows = vec![];
        for key in mode_keys {
            let wpms = &mode_groups[key];
            let best = wpms.iter().copied().fold(0.0_f64, f64::max);
            let avg  = wpms.iter().sum::<f64>() / wpms.len() as f64;
            rows.push((key.clone(), format!("{:.0}  {:.0}", best, avg)));
        }
        sections.push(StatSection { title: "by mode".into(), col_header: Some("best  avg".into()), rows });
    }

    sections
}

pub(crate) fn sections_total_lines(sections: &[StatSection]) -> usize {
    sections.iter().map(|s| s.rows.len() + 3).sum()
}

fn mode_bucket(mode: &str, value: &str) -> String {
    match mode {
        "words" => {
            let n: u64 = value.parse().unwrap_or(0);
            let bucket = match n {
                1..=10    => "1-10",
                11..=25   => "11-25",
                26..=50   => "26-50",
                51..=100  => "51-100",
                101..=250 => "101-250",
                251..=500 => "251-500",
                501..=1000 => "501-1000",
                _          => "1001+",
            };
            format!("words {}", bucket)
        }
        "time" => {
            let n: u64 = value.parse().unwrap_or(0);
            let bucket = match n {
                0..=29   => "<30s",
                30..=60  => "30-60s",
                61..=120 => "61-120s",
                _        => "120s+",
            };
            format!("time {}", bucket)
        }
        // quote values are already short/medium/long/very long
        _ => format!("{} {}", mode, value),
    }
}
