use super::history::TestRecord;

pub(crate) struct RowCache {
    pub(crate) mode:       String,
    pub(crate) wpm:        String,
    pub(crate) raw:        String,
    pub(crate) acc:        String,
    pub(crate) con:        String,
    pub(crate) time:       String,
    pub(crate) done:       &'static str,
    pub(crate) char_stats: String,
    pub(crate) test_num:   String,
}

pub(crate) fn build_row_cache(records: &[TestRecord]) -> Vec<RowCache> {
    let total = records.len();
    records.iter().enumerate().map(|(i, r)| {
        let mode = {
            let mut s = format!("{} {}", r.mode, r.mode_value);
            if r.use_punctuation { s.push_str(" punctuation"); }
            if r.use_numbers     { s.push_str(" numbers"); }
            s
        };
        let fmt_u = |v: Option<usize>| -> String {
            v.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string())
        };
        let char_stats = format!("{}/{}/{}/{}",
            fmt_u(r.correct_chars),
            fmt_u(r.incorrect_chars),
            fmt_u(r.extra_chars),
            fmt_u(r.missed_chars),
        );
        RowCache {
            mode,
            wpm:      r.wpm.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".to_string()),
            raw:      r.raw_wpm.map(|v| format!("{:.0}", v)).unwrap_or_else(|| "-".to_string()),
            acc:      r.accuracy.map(|v| format!("{:.1}%", v)).unwrap_or_else(|| "-".to_string()),
            con:      r.consistency.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "-".to_string()),
            time:     format!("{:.1}s", r.duration_secs),
            done:     if r.completed { "Y" } else { "N" },
            char_stats,
            // index 0 = newest, not oldest. yes, it's backwards. no, don't "fix" it.
            test_num: (total - i).to_string(),
        }
    }).collect()
}

pub(crate) struct DetailCache {
    pub(crate) test_num: usize,
    pub(crate) date:     String,
    pub(crate) fields:   Vec<(&'static str, String)>,
    pub(crate) label_w:  usize,
    pub(crate) value_w:  usize,
}

pub(crate) fn build_detail_cache(
    records:      &[TestRecord],
    record_dates: &[(String, String)],
    selected:     usize,
) -> DetailCache {
    let record   = &records[selected];
    let test_num = records.len() - selected;
    let (date, time_of_day) = &record_dates[selected];

    let mode_str = {
        let mut s = format!("{} {}", record.mode, record.mode_value);
        if record.use_punctuation { s.push_str(" punctuation"); }
        if record.use_numbers     { s.push_str(" numbers"); }
        s
    };

    let fmt_f1 = |v: Option<f64>, suffix: &str| -> String {
        v.map(|x| format!("{:.1}{}", x, suffix)).unwrap_or_else(|| "-".to_string())
    };
    let fmt_f0 = |v: Option<f64>| -> String {
        v.map(|x| format!("{:.0}", x)).unwrap_or_else(|| "-".to_string())
    };
    let fmt_u = |v: Option<usize>| -> String {
        v.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string())
    };

    let char_stats = format!("{}/{}/{}/{}",
        fmt_u(record.correct_chars),
        fmt_u(record.incorrect_chars),
        fmt_u(record.extra_chars),
        fmt_u(record.missed_chars),
    );
    let key_stats = format!("{} correct  {} incorrect  {} total",
        fmt_u(record.correct_keystrokes),
        fmt_u(record.incorrect_keystrokes),
        fmt_u(record.total_keystrokes),
    );

    let fields: Vec<(&'static str, String)> = vec![
        ("date",                  format!("{} {}", date, time_of_day)),
        ("completed",             if record.completed { "yes".into() } else { "no".into() }),
        ("mode",                  mode_str),
        ("language",              record.language.clone()),
        ("duration",              format!("{:.1}s", record.duration_secs)),
        ("wpm",                   fmt_f0(record.wpm)),
        ("raw wpm",               fmt_f0(record.raw_wpm)),
        ("accuracy",              fmt_f1(record.accuracy, "%")),
        ("consistency",           fmt_f1(record.consistency, "%")),
        ("char  cor/inc/ext/mis", char_stats),
        ("keys  cor/inc/total",   key_stats),
        ("quote source",          record.quote_source.clone().unwrap_or_else(|| "-".into())),
    ];

    let label_w = fields.iter().map(|(l, _)| l.len()).max().unwrap_or(10) + 2;
    let value_w = fields.iter().map(|(_, v)| v.len()).max().unwrap_or(10);

    DetailCache { test_num, date: date.clone(), fields, label_w, value_w }
}

pub(crate) struct ColWidthCache {
    pub(crate) max_char_len: usize,
    pub(crate) max_mode_len: usize,
    pub(crate) max_lang_len: usize,
}

pub(crate) fn build_col_width_cache(records: &[TestRecord]) -> ColWidthCache {
    let max_char_len = records.iter()
        .map(|r| {
            let co = r.correct_chars.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
            let ic = r.incorrect_chars.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
            let ex = r.extra_chars.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
            let mi = r.missed_chars.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string());
            format!("{}/{}/{}/{}", co, ic, ex, mi).chars().count()
        })
        .max().unwrap_or(0).max(4);

    let max_mode_len = records.iter()
        .map(|r| {
            let mut s = format!("{} {}", r.mode, r.mode_value);
            if r.use_punctuation { s.push_str(" punctuation"); }
            if r.use_numbers     { s.push_str(" numbers"); }
            s.chars().count()
        })
        .max().unwrap_or(0).max(4);

    let max_lang_len = records.iter()
        .map(|r| r.language.chars().count())
        .max().unwrap_or(0).max(8);

    ColWidthCache { max_char_len, max_mode_len, max_lang_len }
}

#[derive(Clone)]
pub(crate) struct ColumnLayout {
    pub(crate) w_sel:  usize,
    pub(crate) w_num:  usize,
    pub(crate) w_date: usize,
    pub(crate) w_wpm:  usize,
    pub(crate) w_acc:  usize,

    pub(crate) show_mode: bool, pub(crate) w_mode: usize,
    pub(crate) show_lang: bool, pub(crate) w_lang: usize,
    pub(crate) show_raw:  bool, pub(crate) w_raw:  usize,
    pub(crate) show_con:  bool, pub(crate) w_con:  usize,
    pub(crate) show_time: bool, pub(crate) w_time: usize,
    pub(crate) show_char: bool, pub(crate) w_char: usize,
    pub(crate) w_done: usize,
}

// ColWidthCache exists so this function never has to touch records. keep it that way.
pub(crate) fn compute_columns(content_w: usize, cwc: &ColWidthCache) -> ColumnLayout {
    let w_sel  = 2;
    let w_num  = 5;

    let mut w_date = 12;
    let mut w_wpm  = 7;
    let mut w_raw  = 7;
    let mut w_acc  = 9;
    let mut w_con  = 9;
    let mut w_time = 8;

    let mut w_char = cwc.max_char_len + 3;

    let mut w_mode = cwc.max_mode_len + 3;
    let mut w_lang = cwc.max_lang_len + 3;

    let w_done = 5usize; // "done" header + 1 pad

    let mut used = w_sel + w_num + w_date + w_wpm + w_acc + w_done;

    let show_mode = content_w >= used + w_mode; if show_mode { used += w_mode; }
    let show_lang = content_w >= used + w_lang; if show_lang { used += w_lang; }
    let show_raw  = content_w >= used + w_raw;  if show_raw  { used += w_raw;  }
    let show_con  = content_w >= used + w_con;  if show_con  { used += w_con;  }
    let show_time = content_w >= used + w_time; if show_time { used += w_time; }
    let show_char = content_w >= used + w_char; if show_char { used += w_char; }

    let leftover    = content_w.saturating_sub(used);
    let mut n_cols  = 3usize;
    if show_mode { n_cols += 1; }
    if show_lang { n_cols += 1; }
    if show_raw  { n_cols += 1; }
    if show_con  { n_cols += 1; }
    if show_time { n_cols += 1; }
    if show_char { n_cols += 1; }

    let extra = leftover / n_cols;
    let mut rem = leftover % n_cols;
    let mut give = |w: &mut usize| {
        *w += extra + if rem > 0 { rem -= 1; 1 } else { 0 };
    };

    give(&mut w_date);
    if show_mode { give(&mut w_mode); }
    if show_lang { give(&mut w_lang); }
    give(&mut w_wpm);
    if show_raw  { give(&mut w_raw); }
    give(&mut w_acc);
    if show_con  { give(&mut w_con); }
    if show_time { give(&mut w_time); }
    if show_char { give(&mut w_char); }

    ColumnLayout {
        w_sel, w_num, w_date, w_wpm, w_acc,
        show_mode, w_mode,
        show_lang, w_lang,
        show_raw,  w_raw,
        show_con,  w_con,
        show_time, w_time,
        show_char, w_char,
        w_done,
    }
}

pub(crate) fn build_chart_data(records: &[TestRecord]) -> (
    Vec<(f64, f64)>,
    Vec<(f64, f64)>,
    f64,
    Vec<usize>,
) {
    let completed_chrono: Vec<usize> = records.iter().enumerate()
        .filter(|(_, r)| r.completed)
        .map(|(i, _)| i)
        .collect::<Vec<_>>()
        .into_iter().rev().collect();

    let stats_wpm_data: Vec<(f64, f64)> = completed_chrono.iter().enumerate()
        .filter_map(|(i, &ri)| records[ri].wpm.map(|w| (i as f64 + 1.0, w)))
        .collect();

    let max_wpm     = stats_wpm_data.iter().map(|(_, w)| *w).fold(0.0_f64, f64::max);
    let stats_y_max = (max_wpm * 1.2).max(10.0);

    let stats_acc_scaled: Vec<(f64, f64)> = completed_chrono.iter().enumerate()
        .filter_map(|(i, &ri)| {
            records[ri].accuracy.map(|a| (i as f64 + 1.0, (a / 100.0) * stats_y_max))
        })
        .collect();

    (stats_wpm_data, stats_acc_scaled, stats_y_max, completed_chrono)
}
