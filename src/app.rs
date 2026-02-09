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

    pub theme: Theme,

    pub use_numbers: bool,
    pub use_punctuation: bool,

    word_generator: WordGenerator,

    pub input: String,
    pub cursor_idx: usize,
    pub start_time: Option<Instant>,

    pub gross_char_count: usize,
    pub total_errors_ever: usize,
    pub generated_count: usize,
    pub scrolled_word_count: usize,

    pub furthest_word_idx: usize,

    pub st_correct: usize,
    pub st_incorrect: usize,
    pub st_extra: usize,
    pub st_missed: usize,
    pub uncorrected_errors_scrolled: usize,

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
            generated_count: 0,
            scrolled_word_count: 0,
            furthest_word_idx: 0,
            st_correct: 0,
            st_incorrect: 0,
            st_extra: 0,
            st_missed: 0,
            uncorrected_errors_scrolled: 0,
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
            word_data,
            quote_data,
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
        self.generated_count = 0;
        self.scrolled_word_count = 0;
        self.furthest_word_idx = 0;
        self.st_correct = 0;
        self.st_incorrect = 0;
        self.st_extra = 0;
        self.st_missed = 0;
        self.uncorrected_errors_scrolled = 0;
        self.current_quote_source.clear();
        self.show_ui = true;
        self.quote_pool.clear();
        self.total_quote_words = 0;
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
        let duration_min = duration_secs / 60.0;
        let gross_wpm = (self.gross_char_count as f64 / 5.0) / duration_min;
        self.final_raw_wpm = gross_wpm;

        let mut screen_incorrect = 0;
        let mut screen_missed = 0;
        let mut screen_extra = 0;

        for (i, c) in self.input.chars().enumerate() {
            if i < self.display_mask.len() {
                if self.display_mask[i] {
                    screen_extra += 1;
                } else {
                    let target = self.display_string.chars().nth(i).unwrap_or(' ');
                    if c == '\0' { screen_missed += 1; }
                    else if c != target { screen_incorrect += 1; }
                }
            }
        }

        let total_uncorrected = self.uncorrected_errors_scrolled + screen_incorrect + screen_missed + screen_extra;
        let error_rate = total_uncorrected as f64 / duration_min;
        self.final_wpm = (gross_wpm - error_rate).max(0.0);

        if self.gross_char_count > 0 {
            let correct = self.gross_char_count.saturating_sub(self.total_errors_ever);
            self.final_accuracy = (correct as f64 / self.gross_char_count as f64) * 100.0;
        } else { self.final_accuracy = 0.0; }

        self.final_time = duration_secs;
        self.show_ui = true;
    }

    pub fn on_key(&mut self, c: char) {
        if self.state == AppState::Finished { return; }
        if self.state == AppState::Waiting {
            self.start_time = Some(Instant::now());
            self.state = AppState::Running;
        }

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

            if c == ' ' {
                if user_current_word.is_empty() { return; }

                if user_current_word.len() < target_word.len() {
                    let missing_count = target_word.len() - user_current_word.len();
                    for _ in 0..missing_count {
                        self.input.push('\0');
                        self.total_errors_ever += 1;
                        self.cursor_idx += 1;
                    }
                }
            }
        }

        self.show_ui = false;
        self.gross_char_count += 1;

        let target_chars: Vec<char> = self.word_stream_string.chars().collect();
        if self.cursor_idx < target_chars.len() {
            let target_char = target_chars[self.cursor_idx];
            if !strings::are_characters_visually_equal(c, target_char) {
                self.total_errors_ever += 1;
            }
        } else {
            self.total_errors_ever += 1;
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
                if self.input.len() >= self.word_stream_string.len() {
                    self.end_test();
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

        // prevent backspace/space toggle from inflating generated_count
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
        self.update_stream_string();
    }

    fn add_one_word(&mut self) {
        if let Some(new_words) = self.word_generator.add_one_word(
            &self.mode,
            &self.word_stream,
            &mut self.quote_pool,
            self.generated_count,
        ) {
            self.word_stream.extend(new_words);
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
        let options = Options::new(safe_width);
        let lines = textwrap::wrap(&self.display_string, options);
        self.visual_lines = lines.into_iter().map(|c| c.into_owned()).collect();
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

        let input_chars: Vec<char> = self.input.chars().take(chars_to_remove_visual).collect();
        for i in 0..chars_to_remove_visual {
            if i < input_chars.len() && i < self.display_mask.len() {
                let is_extra = self.display_mask[i];
                if is_extra {
                    self.st_extra += 1;
                    self.uncorrected_errors_scrolled += 1;
                } else {
                    let typed = input_chars[i];
                    let target = self.display_string.chars().nth(i).unwrap_or(' ');
                    if typed == '\0' {
                        self.st_missed += 1;
                        self.uncorrected_errors_scrolled += 1;
                    } else if typed == target {
                        self.st_correct += 1;
                    } else {
                        self.st_incorrect += 1;
                        self.uncorrected_errors_scrolled += 1;
                    }
                }
            }
        }

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
}
