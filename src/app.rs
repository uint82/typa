use crate::config::Theme;
use crate::models::{
    AppState, Mode, QuoteData, WordData, Word, WordState
};
use crate::utils::strings;
use crate::generator::WordGenerator;
use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::time::Instant;
use textwrap::Options;
use std::collections::HashSet;

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
    pub final_time: f64,
    pub current_quote_source: String,

    pub word_stream: Vec<Word>,
    pub word_stream_string: String,

    pub terminal_width: u16,
    pub visual_lines: Vec<String>,
    pub display_string: String,
    pub display_mask: Vec<bool>,

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
            final_time: 0.0,
            current_quote_source: String::new(),
            word_stream: Vec::new(),
            word_stream_string: String::new(),
            display_string: String::new(),
            display_mask: Vec::new(),
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

    pub fn quit(&mut self) { self.should_quit = true; }
    pub fn resize(&mut self, width: u16, _height: u16) {
        self.terminal_width = width;
        self.recalculate_lines();
    }
    pub fn on_mouse(&mut self) { if self.state != AppState::Finished { self.show_ui = true; } }

    pub fn restart_test(&mut self) {
        self.input.clear();
        self.cursor_idx = 0;
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

        // in time mode the timer fires mid-word. the untyped remainder of the
        // partial word should not count as missed chars. only judge what was typed.
        if let Mode::Time(_) = self.mode {
            let typed_len = self.input.len();
            if typed_len < self.display_string.len() {
                let truncated: String = self.display_string.chars().take(typed_len).collect();
                self.display_string = truncated;
                self.display_mask.truncate(typed_len);
            }
        }

        let input_ends_with_space = self.input.ends_with(' ');
        let (completed_input, current_word_input) = if input_ends_with_space || self.input.is_empty() {
            (self.input.as_str(), "")
        } else {
            if let Some(last_space_pos) = self.input.rfind(' ') {
                let completed = &self.input[..=last_space_pos];
                let current = &self.input[last_space_pos+1..];
                (completed, current)
            } else {
                ("", self.input.as_str())
            }
        };

        let completed_display_len = completed_input.len();
        let completed_display: String = self.display_string.chars().take(completed_display_len).collect();
        let completed_mask: Vec<bool> = self.display_mask.iter().take(completed_display_len).copied().collect();

        let (_, _, completed_correct_chars, _, _, _) =
            self.calculate_custom_stats_for_slice(completed_input, &completed_display, &completed_mask);

        let current_word_correct_chars = if !current_word_input.is_empty() {
            let input_words: Vec<&str> = self.input.split(' ').collect();
            let current_word_idx = if input_ends_with_space {
                input_words.len().saturating_sub(1)
            } else {
                input_words.len().saturating_sub(1)
            };

            if current_word_idx < self.word_stream.len() {
                let target_word = &self.word_stream[current_word_idx].text;

                let mut has_error = false;
                for (i, c) in current_word_input.chars().enumerate() {
                    if let Some(target_c) = target_word.chars().nth(i) {
                        if c != target_c {
                            has_error = true;
                            break;
                        }
                    } else {
                        has_error = true;
                        break;
                    }
                }

                if !has_error {
                    current_word_input.len()
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        let total_correct_chars = self.st_correct + completed_correct_chars + current_word_correct_chars;

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

        // only record final snapshot if >= 0.495 seconds remain after last full second
        if remaining >= 0.495 {
            self.push_snapshot(duration_secs);
        }
    }

    fn push_snapshot(&mut self, elapsed_secs: f64) {
        if elapsed_secs <= 0.0 { return; }

        let input_ends_with_space = self.input.ends_with(' ');
        let (completed_input, current_word_input) = if input_ends_with_space || self.input.is_empty() {
            (self.input.as_str(), "")
        } else {
            if let Some(last_space_pos) = self.input.rfind(' ') {
                let completed = &self.input[..=last_space_pos];
                let current = &self.input[last_space_pos+1..];
                (completed, current)
            } else {
                ("", self.input.as_str())
            }
        };

        let completed_display_len = completed_input.len();
        let completed_display: String = self.display_string.chars().take(completed_display_len).collect();
        let completed_mask: Vec<bool> = self.display_mask.iter().take(completed_display_len).copied().collect();

        let (_, _, completed_correct_chars, _, _, _) =
            self.calculate_custom_stats_for_slice(completed_input, &completed_display, &completed_mask);

        // if word has ANY error, entire word gets 0. otherwise count correct chars typed so far.
        let current_word_correct_chars = if !current_word_input.is_empty() {
            let input_words: Vec<&str> = self.input.split(' ').collect();
            let current_word_idx = if input_ends_with_space {
                input_words.len().saturating_sub(1)
            } else {
                input_words.len().saturating_sub(1)
            };

            if current_word_idx < self.word_stream.len() {
                let target_word = &self.word_stream[current_word_idx].text;

                let mut has_error = false;
                for (i, c) in current_word_input.chars().enumerate() {
                    if let Some(target_c) = target_word.chars().nth(i) {
                        if c != target_c {
                            has_error = true;
                            break;
                        }
                    } else {
                        has_error = true;
                        break;
                    }
                }

                if !has_error {
                    current_word_input.len()
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        let total_correct_chars = self.st_correct + completed_correct_chars + current_word_correct_chars;
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
            // use floor for cleaner second boundaries
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

            let limit = target_word.len() + 19;
            if user_current_word.len() >= limit {
                if c != ' ' { return; }
            }

            if c != ' ' {
                if self.will_cause_visual_wrap(c) { return; }
            }
        }

        // must track accuracy before cursor manipulation to capture the actual keystroke intent
        self.show_ui = false;
        self.gross_char_count += 1;

        // instead of comparing global indices (which break on extra chars),
        // we compare relative to the current active word.
        let is_keystroke_correct = if word_idx < self.word_stream.len() {
            let target_word = &self.word_stream[word_idx].text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' {
                user_current_word == target_word
            } else {
                if user_current_word.len() < target_word.len() {
                    let target_char = target_word.chars().nth(user_current_word.len()).unwrap_or('\0');
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
        } else {
            self.total_errors_ever += 1;
        }

        if word_idx < self.word_stream.len() {
            let target_word_struct = &self.word_stream[word_idx];
            let target_word = &target_word_struct.text;
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            if c == ' ' {
                if user_current_word.is_empty() { return; }

                let mut is_word_error = false;
                let mut extra_len_penalty = 0;

                if user_current_word != target_word {
                    is_word_error = true;
                }

                if user_current_word.len() > target_word.len() {
                    extra_len_penalty = user_current_word.len() - target_word.len();
                }

                if !self.processed_word_errors.contains(&word_idx) {
                    if is_word_error || extra_len_penalty > 0 {
                        let word_penalty = if is_word_error { 1 } else { 0 };
                        self.total_errors_ever += word_penalty + extra_len_penalty;
                        self.processed_word_errors.insert(word_idx);
                    }
                }

                if user_current_word.len() < target_word.len() {
                    let missing_count = target_word.len() - user_current_word.len();
                    for _ in 0..missing_count {
                        self.input.push('\0');
                        self.cursor_idx += 1;
                    }
                }
            }
        }

        self.input.push(c);
        self.cursor_idx += 1;

        self.sync_display_text();

        if c == ' ' {
            self.on_word_finished();
        }
        self.check_scroll_trigger();

        match self.mode {
            Mode::Words(_) | Mode::Quote(_) => {
                let extra_count = self.display_mask.iter().filter(|&&is_extra| is_extra).count();
                let effective_len = self.input.len().saturating_sub(extra_count);

                if effective_len >= self.word_stream_string.len() {
                    let target_words: Vec<&str> = self.word_stream_string.split(' ').collect();
                    let input_words: Vec<&str> = self.input.split(' ').collect();

                    if let Some(last_target_word) = target_words.last() {
                        let last_word_index = target_words.len() - 1;
                        let last_input_word = input_words.get(last_word_index).unwrap_or(&"");

                        let cleaned_input: String = last_input_word
                            .chars()
                            .filter(|&c| c != '\0')
                            .collect();

                        if &cleaned_input == last_target_word {
                            self.end_test();
                        }
                    }
                }
            }
            _ => {}
        }
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
            self.cursor_idx = self.cursor_idx.saturating_sub(1);
            if popped_char == ' ' || popped_char == '\0' {
                while self.input.ends_with('\0') {
                    self.input.pop();
                    self.cursor_idx = self.cursor_idx.saturating_sub(1);
                }
            }
            self.sync_display_text();
        }
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
            self.word_stream.extend(new_words);
            self.next_word_index = new_next_index;
            if matches!(self.mode, Mode::Words(_)) {
                self.generated_count += 1;
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
        self.sync_display_text();
    }

    fn sync_display_text(&mut self) {
        let clean_chars: Vec<char> = self.word_stream_string.chars().collect();
        let input_chars: Vec<char> = self.input.chars().collect();

        let mut new_display = String::with_capacity(self.word_stream_string.len() + 20);
        let mut new_mask = Vec::with_capacity(self.word_stream_string.len() + 20);

        let mut clean_idx = 0;
        let mut input_idx = 0;

        while clean_idx < clean_chars.len() {
            let clean_char = clean_chars[clean_idx];
            if clean_char == ' ' {
                while input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    new_display.push(input_chars[input_idx]);
                    new_mask.push(true);
                    input_idx += 1;
                }
                new_display.push(' ');
                new_mask.push(false);
                clean_idx += 1;
                if input_idx < input_chars.len() && input_chars[input_idx] == ' ' {
                    input_idx += 1;
                }
            } else {
                new_display.push(clean_char);
                new_mask.push(false);
                clean_idx += 1;
                if input_idx < input_chars.len() && input_chars[input_idx] != ' ' {
                    input_idx += 1;
                }
            }
        }
        self.display_string = new_display;
        self.display_mask = new_mask;
        self.recalculate_lines();
    }

    fn will_cause_visual_wrap(&self, extra_char: char) -> bool {
        let mut candidate_display = self.display_string.clone();
        candidate_display.push(extra_char);
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let safe_width = layout_width.saturating_sub(2);
        let options = Options::new(safe_width);
        let current_lines = textwrap::wrap(&self.display_string, options.clone());
        let candidate_lines = textwrap::wrap(&candidate_display, options);
        let current_cursor_pos = self.input.len();
        let current_line_idx = self.get_line_index_for_cursor(&current_lines, current_cursor_pos);
        let candidate_cursor_pos = current_cursor_pos + 1;
        let candidate_line_idx = self.get_line_index_for_cursor(&candidate_lines, candidate_cursor_pos);
        candidate_line_idx > current_line_idx
    }

    fn get_line_index_for_cursor(&self, lines: &[std::borrow::Cow<'_, str>], cursor_pos: usize) -> usize {
        let mut running_count = 0;
        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1;
            if cursor_pos < running_count + line_len { return i; }
            running_count += line_len;
        }
        if lines.is_empty() { 0 } else { lines.len() - 1 }
    }

    fn recalculate_lines(&mut self) {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let safe_width = layout_width.saturating_sub(2);

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        let words: Vec<&str> = self.display_string.split(' ').collect();

        for word in words.iter() {
            let word_len = word.chars().count();

            let space_before = if current_width == 0 { 0 } else { 1 };
            let total_needed = current_width + space_before + word_len;

            if total_needed <= safe_width {
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

        self.visual_lines = lines;
    }

    fn check_scroll_trigger(&mut self) {
        let mut running_char_count = 0;
        let mut current_line_index = 0;
        for (i, line) in self.visual_lines.iter().enumerate() {
            let line_len = line.len() + 1;
            if self.input.len() < running_char_count + line_len {
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

        let input_chunk: String = self.input.chars().take(chars_to_remove_visual).collect();
        let display_chunk: String = self.display_string.chars().take(chars_to_remove_visual).collect();
        let mask_chunk: Vec<bool> = self.display_mask.iter().take(chars_to_remove_visual).cloned().collect();

        let (acc_cor, acc_inc, raw_cor, raw_inc, raw_ext, raw_mis) =
            self.calculate_custom_stats_for_slice(&input_chunk, &display_chunk, &mask_chunk);

        self.st_correct += raw_cor;
        self.st_incorrect += raw_inc;
        self.st_extra += raw_ext;
        self.st_missed += raw_mis;

        self.acc_score_correct = (self.acc_score_correct + acc_cor).max(0);
        self.acc_score_incorrect = (self.acc_score_incorrect + acc_inc).max(0);

        self.uncorrected_errors_scrolled += raw_inc + raw_mis + raw_ext;

        if self.input.len() >= chars_to_remove_visual {
            let chunk_being_removed = &self.input[..chars_to_remove_visual];
            let words_scrolled = chunk_being_removed.split_whitespace().count();
            self.scrolled_word_count += words_scrolled;

            if words_scrolled > 0 {
                let drain_amount = words_scrolled.min(self.word_stream.len());
                self.word_stream.drain(0..drain_amount);
                self.furthest_word_idx = self.furthest_word_idx.saturating_sub(words_scrolled);
            }
        }

        let mut real_chars_removed = 0;
        for i in 0..chars_to_remove_visual {
            if i < self.display_mask.len() {
                if !self.display_mask[i] { real_chars_removed += 1; }
            }
        }
        if real_chars_removed > 0 {
            if self.word_stream_string.len() >= real_chars_removed {
                self.word_stream_string = self.word_stream_string[real_chars_removed..].to_string();
            }
        }

        if self.cursor_idx >= chars_to_remove_visual {
            self.cursor_idx -= chars_to_remove_visual;
        } else {
            self.cursor_idx = 0;
        }
        if self.input.len() >= chars_to_remove_visual {
            self.input.drain(0..chars_to_remove_visual);
        }
        self.sync_display_text();
    }

    pub fn calculate_custom_stats_for_slice(&self, input_str: &str, display_str: &str, mask: &[bool])
        -> (isize, isize, usize, usize, usize, usize)
    {
        let mut acc_correct_score: isize = 0;
        for &m in mask { if !m { acc_correct_score += 1; } }
        let mut acc_incorrect_score: isize = 0;

        let mut raw_cor = 0;
        let mut raw_inc = 0;
        let mut raw_ext = 0;
        let mut raw_mis = 0;

        let input_chars: Vec<char> = input_str.chars().collect();
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
                    } else if input_char != target_char {
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
                    } else if input_char != target_char {
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
