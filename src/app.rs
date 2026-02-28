use crate::config::Theme;
use crate::history;
use crate::models::{
    AppState, Mode, QuoteData, WordData, Word, WordState
};
use crate::utils::strings;
use crate::generator::WordGenerator;
use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::time::Instant;
use std::collections::{HashMap, HashSet};


#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

pub struct App {
    pub should_quit: bool,
    pub state: AppState,
    pub mode: Mode,
    pub show_ui: bool,

    pub quote_data: QuoteData,
    pub quote_pool: Vec<String>,
    pub total_quote_words: usize,
    pub original_quote_length: usize,

    pub theme: Theme,

    pub use_numbers: bool,
    pub use_punctuation: bool,

    word_generator: WordGenerator,

    pub input: String,
    pub cursor_idx: usize,
    pub start_time: Option<Instant>,

    pub gross_char_count: usize,
    pub total_errors_ever: usize,
    pub processed_word_errors: HashSet<usize>,

    pub generated_count: usize,
    pub scrolled_word_count: usize,

    pub furthest_word_idx: usize,

    pub st_correct: usize,
    pub st_incorrect: usize,
    pub st_extra: usize,
    pub st_missed: usize,

    pub acc_score_correct: isize,
    pub acc_score_incorrect: isize,

    pub uncorrected_errors_scrolled: usize,

    pub live_correct_keystrokes: usize,
    pub live_incorrect_keystrokes: usize,

    pub final_wpm: f64,
    pub final_raw_wpm: f64,
    pub final_accuracy: f64,
    pub final_consistency: f64,
    pub final_time: f64,
    pub current_quote_source: String,

    pub word_stream: Vec<Word>,
    pub word_stream_string: String,

    pub terminal_width: u16,
    pub visual_lines: Vec<String>,
    pub display_string: String,
    pub display_mask: Vec<bool>,
    pub extra_char_count: usize,

    pub missed_chars: HashMap<usize, usize>,
    /// the renderer uses this instead of self.input so missed positions render correctly.
    pub aligned_input: Vec<char>,

    pub word_data: WordData,
    pub next_word_index: usize,

    pub wpm_history: Vec<(f64, f64)>,
    pub raw_wpm_history: Vec<(f64, f64)>,
    pub errors_history: Vec<(f64, f64)>,
    last_snapshot_second: u64,
    prev_incorrect_keystrokes: usize,
}

impl App {
    pub fn new(
        mode: Mode,
        language: String,
        use_numbers: bool,
        use_punctuation: bool,
        theme: Theme,
    ) -> Result<Self> {
        let word_filename = format!("language/{}.json", language);
        let word_file = Asset::get(&word_filename).context(format!(
            "Could not find embedded language file: {}",
            word_filename
        ))?;
        let w_str = std::str::from_utf8(word_file.data.as_ref())?;
        let word_data: WordData = serde_json::from_str(w_str)?;

        let quote_filename = format!("quotes/{}.json", language);
        let quote_file = Asset::get(&quote_filename).context(format!(
            "Could not find embedded quotes file: {}",
            quote_filename
        ))?;
        let q_str = std::str::from_utf8(quote_file.data.as_ref())?;
        let quote_data: QuoteData = serde_json::from_str(q_str)?;

        let word_generator = WordGenerator::new(
            word_data.clone(),
            use_numbers,
            use_punctuation,
        );

        let mut app = Self {
            should_quit: false,
            state: AppState::Waiting,
            mode,
            show_ui: true,
            theme,
            use_numbers,
            use_punctuation,
            word_generator,
            input: String::new(),
            cursor_idx: 0,
            start_time: None,
            gross_char_count: 0,
            total_errors_ever: 0,
            processed_word_errors: HashSet::new(),
            generated_count: 0,
            scrolled_word_count: 0,
            furthest_word_idx: 0,
            st_correct: 0,
            st_incorrect: 0,
            st_extra: 0,
            st_missed: 0,
            acc_score_correct: 0,
            acc_score_incorrect: 0,
            uncorrected_errors_scrolled: 0,
            live_correct_keystrokes: 0,
            live_incorrect_keystrokes: 0,
            final_wpm: 0.0,
            final_raw_wpm: 0.0,
            final_accuracy: 0.0,
            final_consistency: 0.0,
            final_time: 0.0,
            current_quote_source: String::new(),
            word_stream: Vec::new(),
            word_stream_string: String::new(),
            display_string: String::new(),
            display_mask: Vec::new(),
            extra_char_count: 0,
            missed_chars: HashMap::new(),
            aligned_input: Vec::new(),
            terminal_width: 80,
            visual_lines: Vec::new(),
            quote_pool: Vec::new(),
            total_quote_words: 0,
            original_quote_length: 0,
            word_data,
            quote_data,
            next_word_index: 0,
            wpm_history: Vec::new(),
            raw_wpm_history: Vec::new(),
            errors_history: Vec::new(),
            last_snapshot_second: u64::MAX,
            prev_incorrect_keystrokes: 0,
        };

        app.restart_test();
        Ok(app)
    }

    pub fn quit(&mut self) {
        if self.state == AppState::Running {
            let _ = history::record_test(self, false);
        }
        self.should_quit = true;
    }
    pub fn resize(&mut self, width: u16, _height: u16) {
        self.terminal_width = width;
        self.recalculate_lines();
    }
    pub fn on_mouse(&mut self) { if self.state != AppState::Finished { self.show_ui = true; } }

    pub fn restart_test(&mut self) {
        if self.state == AppState::Running {
            let _ = history::record_test(self, false);
        }
        self.input.clear();
        self.cursor_idx = 0;
        self.missed_chars.clear();
        self.aligned_input.clear();
        self.start_time = None;
        self.state = AppState::Waiting;
        self.gross_char_count = 0;
        self.total_errors_ever = 0;
        self.processed_word_errors.clear();
        self.generated_count = 0;
        self.scrolled_word_count = 0;
        self.furthest_word_idx = 0;
        self.st_correct = 0;
        self.st_incorrect = 0;
        self.st_extra = 0;
        self.st_missed = 0;
        self.acc_score_correct = 0;
        self.acc_score_incorrect = 0;
        self.uncorrected_errors_scrolled = 0;
        self.live_correct_keystrokes = 0;
        self.live_incorrect_keystrokes = 0;
        self.current_quote_source.clear();
        self.show_ui = true;
        self.quote_pool.clear();
        self.total_quote_words = 0;
        self.original_quote_length = 0;
        self.next_word_index = 0;
        self.wpm_history.clear();
        self.raw_wpm_history.clear();
        self.errors_history.clear();
        self.last_snapshot_second = u64::MAX;
        self.prev_incorrect_keystrokes = 0;
        self.generate_initial_words();
    }

    pub fn check_time(&mut self) {
        if self.state != AppState::Running { return; }
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            if let Mode::Time(limit) = self.mode {
                 if elapsed >= limit as f64 { self.end_test(); }
            }
        }
    }

    pub fn end_test(&mut self) {
        self.state = AppState::Finished;
        let duration_secs = self.start_time.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);

        // in time mode the timer fires mid-word. untyped chars at the end shouldn't count as missed
        if let Mode::Time(_) = self.mode {
            let typed_len = self.aligned_input.len();
            if typed_len < self.display_string.chars().count() {
                let truncated: String = self.display_string.chars().take(typed_len).collect();
                self.display_string = truncated;
                self.display_mask.truncate(typed_len);
            }
        }

        let total_correct_chars = self.st_correct + self.calculate_live_correct_chars();

        self.final_raw_wpm = (self.gross_char_count as f64 / 5.0) * (60.0 / duration_secs);
        self.final_wpm = (total_correct_chars as f64 / 5.0) * (60.0 / duration_secs);

        let total_keystrokes = self.live_correct_keystrokes + self.live_incorrect_keystrokes;
        if total_keystrokes > 0 {
            self.final_accuracy = (self.live_correct_keystrokes as f64 / total_keystrokes as f64) * 100.0;
        } else {
            self.final_accuracy = 0.0;
        }

        self.final_time = duration_secs;
        self.show_ui = true;

        let last_full_second = if self.last_snapshot_second == u64::MAX {
            0.0
        } else {
            self.last_snapshot_second as f64
        };
        let remaining = duration_secs - last_full_second;

        // skip the final snapshot if it's too close to the last full-second snapshot to avoid a duplicate
        if remaining >= 0.495 {
            self.push_snapshot(duration_secs);
        }

        self.final_consistency = self.calculate_consistency();

        let _ = history::record_test(self, true);
    }

    fn push_snapshot(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 { return; }

        let total_correct_chars = self.st_correct + self.calculate_live_correct_chars();
        let raw_wpm = (self.gross_char_count as f64 / 5.0) * (60.0 / elapsed_secs);
        let net_wpm = (total_correct_chars as f64 / 5.0) * (60.0 / elapsed_secs);

        let errors_this_second = (self.live_incorrect_keystrokes
            .saturating_sub(self.prev_incorrect_keystrokes)) as f64;
        self.prev_incorrect_keystrokes = self.live_incorrect_keystrokes;

        self.wpm_history.push((elapsed_secs, net_wpm));
        self.raw_wpm_history.push((elapsed_secs, raw_wpm));
        self.errors_history.push((elapsed_secs, errors_this_second));
    }

    pub fn record_snapshot_if_needed(&mut self) {
        if self.state != AppState::Running { return; }
        if let Some(start) = self.start_time {
            let elapsed_secs = start.elapsed().as_secs_f64();
            // floor gives clean integer second boundaries instead of rounding artefacts
            let current_second = elapsed_secs.floor() as u64;

            if current_second >= 1 &&
               (self.last_snapshot_second == u64::MAX || current_second > self.last_snapshot_second) {
                self.last_snapshot_second = current_second;
                self.push_snapshot(current_second as f64);
            }
        }
    }

    pub fn on_key(&mut self, c: char) {
        if self.state == AppState::Finished { return; }
        if self.state == AppState::Waiting {
            self.start_time = Some(Instant::now());
            self.state = AppState::Running;
        }

        self.record_snapshot_if_needed();

        let current_input_segments: Vec<&str> = self.input.split(' ').collect();
        let word_idx = current_input_segments.len().saturating_sub(1);

        if word_idx < self.word_stream.len() {
            let target_word_struct = &self.word_stream[word_idx];
            let target_word = &target_word_struct.text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' && user_current_word.is_empty() { return; }

            let target_char_count = target_word.chars().count();
            let user_char_count = user_current_word.chars().count();

            // use char count for the limit, byte len is wrong for multi-byte chars like em dash
            let limit = target_char_count + 19;
            if user_char_count >= limit {
                if c != ' ' { return; }
            }

            if c != ' ' {
                let is_extra = user_char_count >= target_char_count;
                if self.will_cause_visual_wrap(c, is_extra) { return; }
            }
        }

        // accuracy must be recorded before mutating input so we capture the actual intent
        self.show_ui = false;
        self.gross_char_count += 1;

        // compare relative to the current word. global indices break when extra chars shift positions
        let is_keystroke_correct = if word_idx < self.word_stream.len() {
            let target_word = &self.word_stream[word_idx].text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' {
                // word-level visual equality so hyphens typed against em-dash or en-dash counts as correct
                Self::words_visually_equal(user_current_word, target_word)
            } else {
                let user_char_count = user_current_word.chars().count();
                let target_char_count = target_word.chars().count();
                if user_char_count < target_char_count {
                    // use char index, not byte index. target_word may contain multi-byte chars
                    let target_char = target_word.chars().nth(user_char_count).unwrap_or('\0');
                    strings::are_characters_visually_equal(c, target_char)
                } else {
                    false
                }
            }
        } else {
            false
        };

        if is_keystroke_correct {
            self.live_correct_keystrokes += 1;
        } else {
            self.live_incorrect_keystrokes += 1;
        }

        if !is_keystroke_correct {
            self.total_errors_ever += 1;
        }

        if word_idx < self.word_stream.len() {
            if c == ' ' {
                let user_current_word = current_input_segments.last().unwrap_or(&"").to_string();
                self.handle_space_press(word_idx, &user_current_word);
            }
        }

        self.input.push(c);

        if c == ' ' {
            self.on_word_finished();
        }
        self.sync_display_text();
        self.check_scroll_trigger();
        self.check_test_completion();
    }

    pub fn on_backspace(&mut self) {
        if self.state == AppState::Finished { return; }

        if self.input.ends_with(' ') {
            let segments: Vec<&str> = self.input.split(' ').collect();
            if segments.len() >= 2 {
                let last_completed_idx = segments.len() - 2;
                let typed_word = segments[last_completed_idx];

                if let Some(target_word) = self.word_stream.get(last_completed_idx) {
                    if typed_word == target_word.text {
                        return;
                    }
                }

                let current_idx = last_completed_idx + 1;
                if current_idx < self.word_stream.len() {
                    self.word_stream[current_idx].state = WordState::Pending;
                }
                if last_completed_idx < self.word_stream.len() {
                    self.word_stream[last_completed_idx].state = WordState::Active;
                }
            }
        }

        if let Some(popped_char) = self.input.pop() {
            if popped_char == ' ' {
                // clear missed record so the word is treated as fresh when re-typed
                let word_idx = self.input.split(' ').count().saturating_sub(1);
                self.missed_chars.remove(&word_idx);
            }
            self.sync_display_text();
        }
    }

    fn words_visually_equal(typed: &str, target: &str) -> bool {
        let mut t = typed.chars();
        let mut g = target.chars();
        loop {
            let pair = (t.next(), g.next());
            if let (Some(a), Some(b)) = pair {
                if !strings::are_characters_visually_equal(a, b) { return false; }
            } else {
                return pair == (None, None);
            }
        }
    }

    fn handle_space_press(&mut self, word_idx: usize, user_current_word: &str) {
        let target_word = self.word_stream[word_idx].text.clone();

        // visual equality so "-" typed against "—" is not counted as an error
        let is_word_error = !Self::words_visually_equal(user_current_word, &target_word);
        // char counts, byte lengths are wrong for multi-byte chars like "—" (3 bytes, 1 char)
        let user_chars = user_current_word.chars().count();
        let target_chars = target_word.chars().count();
        let extra_len_penalty = user_chars.saturating_sub(target_chars);

        if !self.processed_word_errors.contains(&word_idx) && (is_word_error || extra_len_penalty > 0) {
            let word_penalty = if is_word_error { 1 } else { 0 };
            self.total_errors_ever += word_penalty + extra_len_penalty;
            self.processed_word_errors.insert(word_idx);
        }

        if user_chars < target_chars {
            let missing_count = target_chars - user_chars;
            self.missed_chars.insert(word_idx, missing_count);
        }
    }

    fn check_test_completion(&mut self) {
        match self.mode {
            Mode::Words(_) | Mode::Quote(_) => {
                // subtract extras only. aligned_input includes \0 slots for missed chars
                let effective_len = self.aligned_input.len().saturating_sub(self.extra_char_count);
                if effective_len < self.word_stream_string.chars().count() { return; }

                let target_words: Vec<&str> = self.word_stream_string.split(' ').collect();
                let input_words: Vec<&str> = self.input.split(' ').collect();

                if let Some(last_target_word) = target_words.last() {
                    let last_word_index = target_words.len() - 1;
                    let last_input_word = input_words.get(last_word_index).unwrap_or(&"");
                    if last_input_word == last_target_word {
                        self.end_test();
                    }
                }
            }
            _ => {}
        }
    }

    fn calculate_live_correct_chars(&self) -> usize {
        let ends_with_space = self.aligned_input.last() == Some(&' ');

        let completed_len = if ends_with_space || self.aligned_input.is_empty() {
            self.aligned_input.len()
        } else {
            self.aligned_input.iter().rposition(|&c| c == ' ')
                .map(|p| p + 1)
                .unwrap_or(0)
        };

        let completed_aligned = &self.aligned_input[..completed_len];
        let completed_display: String = self.display_string.chars().take(completed_len).collect();
        let completed_mask: Vec<bool> = self.display_mask.iter().take(completed_len).copied().collect();

        let (_, _, completed_correct_chars, _, _, _) =
            self.calculate_custom_stats_for_slice(completed_aligned, &completed_display, &completed_mask);

        // use self.input for the in-progress word. it has no \0 so indexing is unambiguous
        let current_word_input = if ends_with_space || self.aligned_input.is_empty() {
            ""
        } else if let Some(last_space) = self.input.rfind(' ') {
            &self.input[last_space + 1..]
        } else {
            self.input.as_str()
        };

        let current_word_correct_chars = if !current_word_input.is_empty() {
            let current_word_idx = self.input.split(' ').count().saturating_sub(1);
            if let Some(word) = self.word_stream.get(current_word_idx) {
                let target_word = &word.text;
                let has_error = current_word_input.chars().enumerate().any(|(i, c)| {
                    target_word.chars().nth(i).map_or(true, |tc| !strings::are_characters_visually_equal(c, tc))
                });
                if has_error { 0 } else { current_word_input.chars().count() }
            } else {
                0
            }
        } else {
            0
        };

        completed_correct_chars + current_word_correct_chars
    }

    pub fn resolved_char_stats(&self) -> (usize, usize, usize, usize) {
        let (_, _, vis_cor, vis_inc, vis_ext, vis_mis) =
            self.calculate_custom_stats_for_slice(
                &self.aligned_input,
                &self.display_string,
                &self.display_mask,
            );
        (
            self.st_correct   + vis_cor,
            self.st_incorrect + vis_inc,
            self.st_extra     + vis_ext,
            self.st_missed    + vis_mis,
        )
    }

    fn calculate_consistency(&self) -> f64 {
        let wpms: Vec<f64> = self.wpm_history.iter().map(|(_, w)| *w).collect();
        let n = wpms.len();
        if n < 2 {
            return 100.0;
        }
        let mean = wpms.iter().sum::<f64>() / n as f64;
        let variance = wpms.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / n as f64;
        let std_dev = variance.sqrt();
        (100.0 - std_dev).clamp(0.0, 100.0)
    }

    fn on_word_finished(&mut self) {
        let segments: Vec<&str> = self.input.split(' ').collect();
        let finished_idx = segments.len().saturating_sub(2);

        if finished_idx < self.word_stream.len() {
            self.word_stream[finished_idx].state = WordState::Typed;
        }
        let next_idx = finished_idx + 1;
        if next_idx < self.word_stream.len() {
            self.word_stream[next_idx].state = WordState::Active;
        }

        if finished_idx >= self.furthest_word_idx {
            self.furthest_word_idx = finished_idx + 1;

            let pending_count = self.word_stream.iter()
                .skip(next_idx)
                .filter(|w| w.state == WordState::Pending)
                .count();

            if pending_count < 100 {
                 self.add_one_word();
            }
        }
    }

    fn generate_initial_words(&mut self) {
        let result = self.word_generator.generate_initial_words(&self.mode, &self.quote_data);
        self.word_stream = result.word_stream;
        self.quote_pool = result.quote_pool;
        self.total_quote_words = result.total_quote_words;
        self.current_quote_source = result.current_quote_source;
        self.generated_count = result.generated_count;
        self.next_word_index = result.next_index;
        self.update_stream_string();
        self.sync_display_text();

        if matches!(self.mode, Mode::Quote(_)) {
            self.original_quote_length = self.word_stream_string.chars().count();
        }
    }

    fn add_one_word(&mut self) {
        if let Some((new_words, new_next_index)) = self.word_generator.add_one_word(
            &self.mode,
            &self.word_stream,
            &mut self.quote_pool,
            self.generated_count,
            self.next_word_index,
        ) {
            self.word_stream.extend(new_words.iter().cloned());
            self.next_word_index = new_next_index;
            if matches!(self.mode, Mode::Words(_)) {
                self.generated_count += new_words.len();
            }
            self.update_stream_string();
        }
    }

    fn update_stream_string(&mut self) {
        self.word_stream_string = self.word_stream
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<&str>>()
            .join(" ");
    }

    fn sync_display_text(&mut self) {
        let clean_chars: Vec<char> = self.word_stream_string.chars().collect();
        let input_chars: Vec<char> = self.input.chars().collect();

        let mut new_display = String::with_capacity(self.word_stream_string.len() + 20);
        let mut new_mask: Vec<bool> = Vec::with_capacity(self.word_stream_string.len() + 20);
        let mut new_aligned: Vec<char> = Vec::with_capacity(self.word_stream_string.len() + 20);

        let mut clean_idx = 0;
        let mut input_idx = 0;
        let mut word_idx = 0usize;

        while clean_idx < clean_chars.len() {
            let clean_char = clean_chars[clean_idx];
            if clean_char == ' ' {
                while input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    new_display.push(input_chars[input_idx]);
                    new_mask.push(true);
                    new_aligned.push(input_chars[input_idx]);
                    input_idx += 1;
                }
                if input_idx < input_chars.len() && input_chars[input_idx] == ' ' {
                    // inject \0 slots so aligned_input has the right length for missed positions
                    if let Some(&missed) = self.missed_chars.get(&word_idx) {
                        for _ in 0..missed {
                            new_aligned.push('\0');
                        }
                    }
                    new_display.push(' ');
                    new_mask.push(false);
                    new_aligned.push(' ');
                    input_idx += 1;
                    word_idx += 1;
                } else {
                    new_display.push(' ');
                    new_mask.push(false);
                }
                clean_idx += 1;
            } else {
                new_display.push(clean_char);
                new_mask.push(false);
                clean_idx += 1;
                if input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    new_aligned.push(input_chars[input_idx]);
                    input_idx += 1;
                }
            }
        }

        self.display_string = new_display;
        self.display_mask = new_mask;
        self.aligned_input = new_aligned;
        self.extra_char_count = self.display_mask.iter().filter(|&&x| x).count();
        // cursor_idx mirrors aligned_input length so callers never need to track it manually
        self.cursor_idx = self.aligned_input.len();
        self.recalculate_lines();
    }

    fn will_cause_visual_wrap(&self, extra_char: char, is_extra: bool) -> bool {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        // extra chars use the full width. no caret buffer needed since they trail behind it
        let candidate_width = if is_extra { layout_width } else { layout_width.saturating_sub(2) };

        let current_line_idx = Self::line_idx_for_cursor(&self.visual_lines, self.aligned_input.len());

        let mut candidate_display = self.display_string.clone();
        candidate_display.push(extra_char);
        let candidate_lines = Self::wrap_into_lines(&candidate_display, candidate_width);
        let candidate_line_idx = Self::line_idx_for_cursor(&candidate_lines, self.aligned_input.len() + 1);

        if is_extra {
            candidate_line_idx > current_line_idx
        } else {
            candidate_line_idx >= 3
        }
    }

    /// used by both recalculate_lines and will_cause_visual_wrap so they always agree on boundaries
    fn wrap_into_lines(text: &str, width: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0usize;

        for word in text.split(' ') {
            let word_len = word.chars().count();
            let space_before = if current_width == 0 { 0 } else { 1 };

            if current_width + space_before + word_len <= width {
                if current_width > 0 {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_len;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                }
                current_line.clear();
                current_line.push_str(word);
                current_width = word_len;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    fn line_idx_for_cursor(lines: &[String], cursor_pos: usize) -> usize {
        let mut running = 0usize;
        for (i, line) in lines.iter().enumerate() {
            let line_len = line.chars().count() + 1; // +1 accounts for the space that separates lines
            if cursor_pos < running + line_len { return i; }
            running += line_len;
        }
        if lines.is_empty() { 0 } else { lines.len() - 1 }
    }

    fn recalculate_lines(&mut self) {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let safe_width = layout_width.saturating_sub(2);
        self.visual_lines = Self::wrap_into_lines(&self.display_string, safe_width);
    }

    fn check_scroll_trigger(&mut self) {
        let mut running_char_count = 0;
        let mut current_line_index = 0;
        for (i, line) in self.visual_lines.iter().enumerate() {
            let line_len = line.chars().count() + 1;
            if self.aligned_input.len() < running_char_count + line_len {
                current_line_index = i;
                break;
            }
            running_char_count += line_len;
        }
        if current_line_index >= 2 {
            self.delete_first_visual_line();
        }
    }

    fn delete_first_visual_line(&mut self) {
        if self.visual_lines.is_empty() { return; }
        let first_line = &self.visual_lines[0];
        let visual_char_count = first_line.chars().count();
        let mut chars_to_remove_visual = visual_char_count;

        if self.display_string.chars().count() > visual_char_count {
            if let Some(c) = self.display_string.chars().nth(visual_char_count) {
                if c == ' ' { chars_to_remove_visual += 1; }
            }
        }

        // aligned_input has \0 for missed positions, so stats are accurate even for short words
        let capped = chars_to_remove_visual.min(self.aligned_input.len());
        let aligned_chunk = &self.aligned_input[..capped];
        let display_chunk: String = self.display_string.chars().take(chars_to_remove_visual).collect();
        let mask_chunk: Vec<bool> = self.display_mask.iter().take(chars_to_remove_visual).cloned().collect();

        let (acc_cor, acc_inc, raw_cor, raw_inc, raw_ext, raw_mis) =
            self.calculate_custom_stats_for_slice(aligned_chunk, &display_chunk, &mask_chunk);

        self.st_correct += raw_cor;
        self.st_incorrect += raw_inc;
        self.st_extra += raw_ext;
        self.st_missed += raw_mis;

        self.acc_score_correct = (self.acc_score_correct + acc_cor).max(0);
        self.acc_score_incorrect = (self.acc_score_incorrect + acc_inc).max(0);

        self.uncorrected_errors_scrolled += raw_inc + raw_mis + raw_ext;

        let tokens_scrolled = aligned_chunk.iter().filter(|&&c| c == ' ').count();
        if tokens_scrolled > 0 {
            self.scrolled_word_count += tokens_scrolled;
            let drain_amount = tokens_scrolled.min(self.word_stream.len());
            self.word_stream.drain(0..drain_amount);
            self.furthest_word_idx = self.furthest_word_idx.saturating_sub(tokens_scrolled);

            // word indices shift down after scrolling, so remap both maps to stay in sync
            self.missed_chars = self.missed_chars
                .iter()
                .filter(|(&k, _)| k >= tokens_scrolled)
                .map(|(&k, &v)| (k - tokens_scrolled, v))
                .collect();

            self.processed_word_errors = self.processed_word_errors
                .iter()
                .filter(|&&k| k >= tokens_scrolled)
                .map(|&k| k - tokens_scrolled)
                .collect();
        }

        let mut real_chars_removed = 0;
        for i in 0..chars_to_remove_visual {
            if i < self.display_mask.len() {
                if !self.display_mask[i] { real_chars_removed += 1; }
            }
        }
        if real_chars_removed > 0 {
            // real_chars_removed is a char count. must convert to byte offset before slicing
            let ws_byte_len: usize = self.word_stream_string.chars()
                .take(real_chars_removed)
                .map(|c| c.len_utf8())
                .sum();
            if self.word_stream_string.len() >= ws_byte_len {
                self.word_stream_string = self.word_stream_string[ws_byte_len..].to_string();
            }
        }

        // self.input has no \0 so we count real chars from aligned_chunk to know how many to drain
        let clean_chars_to_remove = aligned_chunk.iter().filter(|&&c| c != '\0').count();
        let byte_len: usize = self.input.chars().take(clean_chars_to_remove).map(|c| c.len_utf8()).sum();
        self.input.drain(..byte_len);

        self.sync_display_text();
    }

    pub fn calculate_custom_stats_for_slice(&self, input_chars: &[char], display_str: &str, mask: &[bool])
        -> (isize, isize, usize, usize, usize, usize)
    {
        let mut acc_correct_score: isize = 0;
        for &m in mask { if !m { acc_correct_score += 1; } }
        let mut acc_incorrect_score: isize = 0;

        let mut raw_cor = 0;
        let mut raw_inc = 0;
        let mut raw_ext = 0;
        let mut raw_mis = 0;

        let display_chars: Vec<char> = display_str.chars().collect();

        let mut i = 0;
        while i < display_chars.len() {
            let mut word_end = i;
            while word_end < display_chars.len() {
                let is_extra = if word_end < mask.len() { mask[word_end] } else { false };
                if !is_extra && display_chars[word_end] == ' ' { break; }
                word_end += 1;
            }

            let mut word_has_error = false;

            for k in i..word_end {
                let is_extra = if k < mask.len() { mask[k] } else { false };
                let target_char = display_chars[k];
                let input_char = input_chars.get(k).copied().unwrap_or('\0');

                if is_extra {
                    word_has_error = true;
                } else {
                    if input_char == '\0' {
                        word_has_error = true;
                    } else if !strings::are_characters_visually_equal(input_char, target_char) {
                        word_has_error = true;
                    }
                }
            }

            for k in i..word_end {
                let is_extra = if k < mask.len() { mask[k] } else { false };
                let target_char = display_chars[k];
                let input_char = input_chars.get(k).copied().unwrap_or('\0');

                if is_extra {
                    acc_incorrect_score += 1;
                    raw_ext += 1;
                } else {
                    if input_char == '\0' {
                        acc_correct_score -= 1;
                        raw_mis += 1;
                    } else if !strings::are_characters_visually_equal(input_char, target_char) {
                        acc_correct_score -= 1;
                        acc_incorrect_score += 1;
                        raw_inc += 1;
                    } else {
                        if !word_has_error {
                            raw_cor += 1;
                        }
                    }
                }
            }

            if word_end < display_chars.len() {
                if word_has_error {
                    acc_correct_score -= 1;
                    acc_incorrect_score += 1;
                } else {
                    raw_cor += 1;
                }
                i = word_end + 1;
            } else {
                i = word_end;
            }
        }

        (acc_correct_score, acc_incorrect_score, raw_cor, raw_inc, raw_ext, raw_mis)
    }
}
