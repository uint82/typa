use crate::config::Theme;
use crate::utils::strings;
use anyhow::{Context, Result};
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::Rng;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::time::Instant;
use textwrap::Options;

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Asset;

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteLength {
    Short,
    Medium,
    Long,
    VeryLong,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuoteSelector {
    Category(QuoteLength),
    Id(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Time(u64),
    Words(usize),
    Quote(QuoteSelector),
}

#[derive(Debug, PartialEq)]
pub enum AppState {
    Waiting,
    Running,
    Finished,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuoteEntry {
    pub text: String,
    pub source: String,
    pub length: usize,
    pub id: usize,
}
#[derive(Debug, Deserialize, Clone)]
pub struct QuoteData {
    #[allow(dead_code)]
    pub language: String,
    pub groups: Vec<Vec<usize>>,
    pub quotes: Vec<QuoteEntry>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct WordData {
    #[allow(dead_code)]
    pub name: String,
    pub words: Vec<String>,
}

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

    pub input: String,
    pub cursor_idx: usize,
    pub start_time: Option<Instant>,

    pub gross_char_count: usize,
    pub total_errors_ever: usize,
    pub generated_count: usize,
    pub scrolled_word_count: usize,

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

    pub word_stream: Vec<String>,
    pub word_stream_string: String,

    pub terminal_width: u16,
    pub visual_lines: Vec<String>,
    // the text currently displayed on screen (includes extra "ghost" chars)
    pub display_string: String,
    // maps 1:1 to display_string. true if the char is an "extra" inserted error.
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

        let mut app = Self {
            should_quit: false,
            state: AppState::Waiting,
            mode,
            show_ui: true,

            theme,

            use_numbers,
            use_punctuation,

            input: String::new(),
            cursor_idx: 0,
            start_time: None,

            gross_char_count: 0,
            total_errors_ever: 0,
            generated_count: 0,
            scrolled_word_count: 0,

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

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn resize(&mut self, width: u16, _height: u16) {
        self.terminal_width = width;
        self.recalculate_lines();
    }

    pub fn on_mouse(&mut self) {
        if self.state != AppState::Finished {
            self.show_ui = true;
        }
    }

    pub fn restart_test(&mut self) {
        self.input.clear();
        self.cursor_idx = 0;
        self.start_time = None;
        self.state = AppState::Waiting;

        self.gross_char_count = 0;
        self.total_errors_ever = 0;
        self.generated_count = 0;
        self.scrolled_word_count = 0;

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
        if self.state != AppState::Running {
            return;
        }
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed().as_secs_f64();
            match self.mode {
                Mode::Time(limit) => {
                    if elapsed >= limit as f64 {
                        self.end_test();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn end_test(&mut self) {
        self.state = AppState::Finished;
        let duration_secs = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(1.0);
        let duration_min = duration_secs / 60.0;

        let gross_wpm = (self.gross_char_count as f64 / 5.0) / duration_min;
        self.final_raw_wpm = gross_wpm;

        // calculate current screen errors + scrolled errors for WPM
        // use the display_mask based counting now for higher accuracy
        let mut screen_incorrect = 0;
        let mut screen_missed = 0;
        let mut screen_extra = 0;

        for (i, c) in self.input.chars().enumerate() {
            if i < self.display_mask.len() {
                let is_extra = self.display_mask[i];
                if is_extra {
                    screen_extra += 1;
                } else {
                    let target = self.display_string.chars().nth(i).unwrap_or(' ');
                    if c == '\0' {
                        screen_missed += 1;
                    } else if c != target {
                        screen_incorrect += 1;
                    }
                }
            }
        }

        let total_uncorrected =
            self.uncorrected_errors_scrolled + screen_incorrect + screen_missed + screen_extra;

        let error_rate = total_uncorrected as f64 / duration_min;
        self.final_wpm = (gross_wpm - error_rate).max(0.0);

        if self.gross_char_count > 0 {
            let correct_entries = self.gross_char_count.saturating_sub(self.total_errors_ever);
            self.final_accuracy = (correct_entries as f64 / self.gross_char_count as f64) * 100.0;
        } else {
            self.final_accuracy = 0.0;
        }

        self.final_time = duration_secs;
        self.show_ui = true;
    }

    pub fn on_key(&mut self, c: char) {
        if self.state == AppState::Finished {
            return;
        }

        if self.state == AppState::Waiting {
            self.start_time = Some(Instant::now());
            self.state = AppState::Running;
        }

        let current_input_segments: Vec<&str> = self.input.split(' ').collect();
        let word_idx = current_input_segments.len().saturating_sub(1);

        if word_idx < self.word_stream.len() {
            let target_word = &self.word_stream[word_idx];
            let user_current_word = current_input_segments.last().unwrap_or(&"");

            // max 19 extra chars
            let limit = target_word.len() + 19;
            if user_current_word.len() >= limit {
                if c != ' ' {
                    return;
                }
            }

            // don't allow non-space chars to force a line wrap
            if c != ' ' {
                if self.will_cause_visual_wrap(c) {
                    return;
                }
            }

            if c == ' ' {
                // require at least 1 char typed before spacebar is allowed
                if user_current_word.is_empty() {
                    return;
                }

                if user_current_word.len() < target_word.len() {
                    let missing_count = target_word.len() - user_current_word.len();
                    for _ in 0..missing_count {
                        self.input.push('\0'); // marker for missed chars
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
        if self.state == AppState::Finished {
            return;
        }

        // prevent backspace over a perfectly typed word.
        if self.input.ends_with(' ') {
            let segments: Vec<&str> = self.input.split(' ').collect();
            if segments.len() >= 2 {
                let last_completed_idx = segments.len() - 2;
                let typed_word = segments[last_completed_idx];
                if let Some(target_word) = self.word_stream.get(last_completed_idx) {
                    if typed_word == target_word {
                        return;
                    }
                }
            }
        }

        if let Some(popped_char) = self.input.pop() {
            self.cursor_idx = self.cursor_idx.saturating_sub(1);

            // if we deleted a space (' ') OR a ghost char ('\0'), check if there are
            // more ghost chars immediately behind it. if so, delete them all.
            if popped_char == ' ' || popped_char == '\0' {
                while self.input.ends_with('\0') {
                    self.input.pop();
                    self.cursor_idx = self.cursor_idx.saturating_sub(1);
                }
            }

            self.sync_display_text();
        }
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
        let candidate_line_idx =
            self.get_line_index_for_cursor(&candidate_lines, candidate_cursor_pos);

        candidate_line_idx > current_line_idx
    }

    fn get_line_index_for_cursor(
        &self,
        lines: &[std::borrow::Cow<'_, str>],
        cursor_pos: usize,
    ) -> usize {
        let mut running_count = 0;
        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1;
            if cursor_pos < running_count + line_len {
                return i;
            }
            running_count += line_len;
        }
        if lines.is_empty() {
            0
        } else {
            lines.len() - 1
        }
    }

    fn recalculate_lines(&mut self) {
        let layout_width = (self.terminal_width as usize * 80) / 100;
        let safe_width = layout_width.saturating_sub(2);

        let options = Options::new(safe_width);
        let lines = textwrap::wrap(&self.display_string, options);
        self.visual_lines = lines.into_iter().map(|c| c.into_owned()).collect();
    }

    fn on_word_finished(&mut self) {
        self.add_one_word();
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
        if self.visual_lines.is_empty() {
            return;
        }

        let first_line = &self.visual_lines[0];
        let visual_char_count = first_line.chars().count();

        let mut chars_to_remove_visual = visual_char_count;

        if self.display_string.chars().count() > visual_char_count {
            if let Some(c) = self.display_string.chars().nth(visual_char_count) {
                if c == ' ' {
                    chars_to_remove_visual += 1;
                }
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
            self.scrolled_word_count += chunk_being_removed.split_whitespace().count();
        }

        let mut real_chars_removed = 0;
        for i in 0..chars_to_remove_visual {
            if i < self.display_mask.len() {
                if !self.display_mask[i] {
                    real_chars_removed += 1;
                }
            }
        }

        if real_chars_removed > 0 {
            if self.word_stream_string.len() >= real_chars_removed {
                let remainder = self.word_stream_string[real_chars_removed..].to_string();
                self.word_stream_string = remainder;

                self.word_stream = self
                    .word_stream_string
                    .split_whitespace()
                    .map(String::from)
                    .collect();
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

    fn generate_unique_batch(&self, count: usize, rng: &mut impl Rng) -> Vec<String> {
        let mut final_stream: Vec<String> = Vec::with_capacity(count);
        let mut deck = self.word_data.words.clone();
        while final_stream.len() < count {
            deck.shuffle(rng);
            for (i, word_ref) in deck.iter().enumerate() {
                if final_stream.len() >= count {
                    break;
                }
                let mut word_str = word_ref.clone();
                if let Some(last_word) = final_stream.last() {
                    if last_word.contains(&word_str) || word_str == *last_word {
                        if i + 1 < deck.len() {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
                if self.use_numbers && rng.random_bool(0.15) {
                    final_stream.push(rng.random_range(0..=9999).to_string());
                    continue;
                }
                if self.use_punctuation {
                    if rng.random_bool(0.40) {
                        word_str = self.apply_contraction_logic(word_str, rng);
                    }
                    if rng.random_bool(0.30) {
                        let p_type = rng.random_range(0..100);
                        match p_type {
                            0..=39 => word_str.push(','),
                            40..=69 => word_str.push('.'),
                            70..=74 => word_str.push('!'),
                            80..=89 => word_str = format!("\"{}\"", word_str),
                            90..=94 => word_str = format!("'{}'", word_str),
                            95..=99 => word_str = format!("({})", word_str),
                            _ => {}
                        }
                    }
                }
                final_stream.push(word_str);
            }
        }
        final_stream
    }

    fn generate_smart_word(&self, rng: &mut impl Rng) -> Vec<String> {
        if self.use_numbers && rng.random_bool(0.15) {
            return vec![rng.random_range(0..=9999).to_string()];
        }
        let mut word = if let Some(w) = self.word_data.words.choose(rng) {
            w.clone()
        } else {
            "word".to_string()
        };
        if self.use_punctuation {
            if rng.random_bool(0.40) {
                word = self.apply_contraction_logic(word, rng);
            }
            if rng.random_bool(0.30) {
                let p_type = rng.random_range(0..100);
                match p_type {
                    0..=39 => word.push(','),
                    40..=69 => word.push('.'),
                    70..=74 => word.push('!'),
                    75..=79 => return vec![word, "-".to_string()],
                    80..=89 => word = format!("\"{}\"", word),
                    90..=94 => word = format!("'{}'", word),
                    95..=99 => word = format!("({})", word),
                    _ => {}
                }
            }
        }
        vec![word]
    }

    fn apply_contraction_logic(&self, original: String, rng: &mut impl Rng) -> String {
        let lower = original.to_lowercase();
        if let Some(replacements) = Self::get_contraction_replacements(&lower) {
            if let Some(replacement) = replacements.choose(rng) {
                return self.match_casing(&original, replacement);
            }
        }
        original
    }

    fn get_contraction_replacements(word: &str) -> Option<&'static [&'static str]> {
        match word {
            "are" => Some(&["aren't"]),
            "can" => Some(&["can't"]),
            "could" => Some(&["couldn't"]),
            "did" => Some(&["didn't"]),
            "does" => Some(&["doesn't"]),
            "do" => Some(&["don't"]),
            "had" => Some(&["hadn't"]),
            "has" => Some(&["hasn't"]),
            "have" => Some(&["haven't"]),
            "is" => Some(&["isn't"]),
            "it" => Some(&["it's", "it'll"]),
            "i" => Some(&["i'm", "i'll", "i've", "i'd"]),
            "you" => Some(&["you'll", "you're", "you've", "you'd"]),
            "that" => Some(&["that's", "that'll", "that'd"]),
            "must" => Some(&["mustn't", "must've"]),
            "there" => Some(&["there's", "there'll", "there'd"]),
            "he" => Some(&["he's", "he'll", "he'd"]),
            "she" => Some(&["she's", "she'll", "she'd"]),
            "we" => Some(&["we're", "we'll", "we'd"]),
            "they" => Some(&["they're", "they'll", "they'd"]),
            "should" => Some(&["shouldn't", "should've"]),
            "was" => Some(&["wasn't"]),
            "were" => Some(&["weren't"]),
            "will" => Some(&["won't"]),
            "would" => Some(&["wouldn't", "would've"]),
            "going" => Some(&["goin'"]),
            _ => None,
        }
    }

    fn match_casing(&self, original: &str, replacement: &str) -> String {
        let is_all_upper = original
            .chars()
            .all(|c| !c.is_alphabetic() || c.is_uppercase());
        if is_all_upper {
            return replacement.to_uppercase();
        }
        let first_is_upper = original.chars().next().map_or(false, |c| c.is_uppercase());
        if first_is_upper {
            let mut c = replacement.chars();
            match c.next() {
                Option::None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        } else {
            replacement.to_string()
        }
    }

    fn generate_initial_words(&mut self) {
        let mut rng = rand::rng();
        self.word_stream.clear();

        match &self.mode {
            Mode::Time(_) => {
                for _ in 0..100 {
                    let words = self.generate_smart_word(&mut rng);
                    self.word_stream.extend(words);
                }
            }
            Mode::Words(count) => {
                // cap initial size at 100 to prevent lag
                let limit = (*count).min(100);
                let words = self.generate_unique_batch(limit, &mut rng);
                self.generated_count = words.len();
                self.word_stream = words;
            }
            Mode::Quote(selector) => {
                let q_opt = match selector {
                    QuoteSelector::Id(target_id) => {
                        self.quote_data.quotes.iter().find(|q| q.id == *target_id)
                    }
                    QuoteSelector::Category(len_category) => {
                        let range = match len_category {
                            QuoteLength::Short => &self.quote_data.groups[0],
                            QuoteLength::Medium => &self.quote_data.groups[1],
                            QuoteLength::Long => &self.quote_data.groups[2],
                            QuoteLength::VeryLong => &self.quote_data.groups[3],
                            QuoteLength::All => &vec![0, 9999],
                        };
                        let valid: Vec<&QuoteEntry> = self
                            .quote_data
                            .quotes
                            .iter()
                            .filter(|q| q.length >= range[0] && q.length <= range[1])
                            .collect();
                        valid.choose(&mut rng).copied()
                    }
                };

                if let Some(q) = q_opt {
                    let clean_text = strings::clean_typography_symbols(&q.text);
                    let all_words: Vec<String> = clean_text.split_whitespace().map(String::from).collect();

                    self.total_quote_words = all_words.len();
                    self.current_quote_source = q.source.clone();

                    if all_words.len() > 100 {
                        self.word_stream = all_words[..100].to_vec();
                        let mut pool = all_words[100..].to_vec();
                        pool.reverse();
                        self.quote_pool = pool;
                    } else {
                        self.word_stream = all_words;
                        self.quote_pool = Vec::new();
                    }
                } else {
                    self.word_stream = vec!["No".to_string(), "Quote".to_string(), "Found".to_string()];
                    self.total_quote_words = 3;
                    self.quote_pool = Vec::new();
                }
            }
        }

        if self.use_punctuation && !matches!(self.mode, Mode::Quote(_)) {
            self.apply_sentence_rules();
            if let Some(last) = self.word_stream.last() {
                if last == "-" {
                    self.word_stream.pop();
                }
            }
            if let Some(last) = self.word_stream.last_mut() {
                if last.ends_with(',') {
                    last.pop();
                }
                let c = last.chars().last().unwrap_or(' ');
                if !['.', '!', '?'].contains(&c) {
                    last.push('.');
                }
            }
        }
        self.update_stream_string();
    }

    fn add_one_word(&mut self) {
        let mut rng = rand::rng();

        match self.mode {
            Mode::Time(_) => {
                let mut new_words = self.generate_smart_word(&mut rng);
                if self.use_punctuation {
                    if let Some(first_new_word) = new_words.first_mut() {
                        if let Some(last_word) = self.word_stream.last() {
                            if Self::ends_with_terminator(last_word) {
                                Self::capitalize_word(first_new_word);
                            }
                        }
                    }
                }
                self.word_stream.extend(new_words);
                self.update_stream_string();
            }
            Mode::Quote(_) => {
                if let Some(next_word) = self.quote_pool.pop() {
                    self.word_stream.push(next_word);
                    self.update_stream_string();
                }
            }
            Mode::Words(target) => {
                if self.generated_count < target {
                    let mut new_words = self.generate_smart_word(&mut rng);

                    // optional: basic duplicate prevention
                    if let Some(last) = self.word_stream.last() {
                        if let Some(first_new) = new_words.first() {
                            if last == first_new {
                                new_words = self.generate_smart_word(&mut rng);
                            }
                        }
                    }

                    if self.use_punctuation {
                        if let Some(first_new_word) = new_words.first_mut() {
                            if let Some(last_word) = self.word_stream.last() {
                                if Self::ends_with_terminator(last_word) {
                                    Self::capitalize_word(first_new_word);
                                }
                            }
                        }
                    }

                    self.word_stream.extend(new_words);
                    self.generated_count += 1;
                    self.update_stream_string();
                }
            }
        }
    }

    fn apply_sentence_rules(&mut self) {
        if self.word_stream.is_empty() {
            return;
        }
        if let Some(first) = self.word_stream.first_mut() {
            Self::capitalize_word(first);
        }
        let len = self.word_stream.len();
        for i in 0..len - 1 {
            let should_cap = Self::ends_with_terminator(&self.word_stream[i]);
            if should_cap {
                Self::capitalize_word(&mut self.word_stream[i + 1]);
            }
        }
    }

    fn capitalize_word(w: &mut String) {
        if let Some(idx) = w.find(|c: char| c.is_alphabetic()) {
            let mut chars: Vec<char> = w.chars().collect();
            if let Some(c) = chars.get_mut(idx) {
                *c = c.to_uppercase().next().unwrap_or(*c);
            }
            *w = chars.into_iter().collect();
        }
    }

    fn ends_with_terminator(w: &str) -> bool {
        w.contains('.') || w.contains('!') || w.contains('?')
    }

    fn update_stream_string(&mut self) {
        self.word_stream_string = self.word_stream.join(" ");
        self.sync_display_text();
    }
}
