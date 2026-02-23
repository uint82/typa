use crate::models::{Mode, QuoteData, WordData, Word, WordState};
use super::formatting;
use super::punctuation::PunctuationRules;
use super::sourcing::TextSource;
use super::quote_controller;
use super::word_controller;

pub struct WordGenerator {
    source: TextSource,
    rules: PunctuationRules,
}

pub struct GeneratedWords {
    pub word_stream: Vec<Word>,
    pub quote_pool: Vec<String>,
    pub total_quote_words: usize,
    pub current_quote_source: String,
    pub generated_count: usize,
    pub next_index: usize,
}

impl WordGenerator {
    pub fn new(word_data: WordData, use_numbers: bool, use_punctuation: bool) -> Self {
        Self {
            source: TextSource::new(word_data),
            rules: PunctuationRules {
                use_numbers,
                use_punctuation,
            },
        }
    }

    pub fn generate_initial_words(
        &self,
        mode: &Mode,
        quote_data: &QuoteData,
    ) -> GeneratedWords {
        let mut rng = rand::rng();

        let mut quote_pool = Vec::new();
        let mut total_quote_words = 0;
        let mut current_quote_source = String::new();
        let mut generated_count = 0;

        let mut raw_stream = match mode {
            Mode::Time(_) => {
                word_controller::generate_time_batch(&self.source, &self.rules, &mut rng)
            }
            Mode::Words(count) => {
                let (stream, _) = word_controller::generate_count_batch(&self.source, &self.rules, *count, &mut rng);
                stream
            }
            Mode::Quote(selector) => {
                let result = quote_controller::generate(&self.source, selector, quote_data, &mut rng);
                quote_pool = result.quote_pool;
                total_quote_words = result.total_words;
                current_quote_source = result.source_text;
                result.word_stream
            }
        };

        if self.rules.use_punctuation && !matches!(mode, Mode::Quote(_)) {
            formatting::finalize_stream_punctuation(&mut raw_stream);
        }

        // count all tokens after finalization (em dashes count as words toward the limit)
        if matches!(mode, Mode::Words(_)) {
            generated_count = raw_stream.len();
        }

        let word_stream: Vec<Word> = raw_stream
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let mut w = Word::new(text, i);
                if i == 0 {
                    w.state = WordState::Active;
                }
                w
            })
            .collect();

        let next_index = word_stream.len();

        GeneratedWords {
            word_stream,
            quote_pool,
            total_quote_words,
            current_quote_source,
            generated_count,
            next_index,
        }
    }

    pub fn add_one_word(
        &self,
        mode: &Mode,
        existing_stream: &[Word],
        quote_pool: &mut Vec<String>,
        generated_count: usize,
        next_index: usize,
    ) -> Option<(Vec<Word>, usize)> {
        let mut rng = rand::rng();

        let context_strings: Vec<String> = existing_stream.iter().map(|w| w.text.clone()).collect();

        let new_raw_words = match mode {
            Mode::Time(_) => {
                let is_sentence_start = context_strings.last()
                    .map(|w| word_controller::is_sentence_end_pub(w))
                    .unwrap_or(true);
                let ctx = word_controller::build_context_pub(&context_strings);
                let mut new_words = word_controller::generate_smart_word(&self.source, &self.rules, &mut rng, is_sentence_start, &ctx);
                formatting::apply_contextual_capitalization(&mut new_words, &context_strings, self.rules.use_punctuation);
                Some(new_words)
            }
            Mode::Quote(_) => {
                quote_controller::next_word(quote_pool)
            },
            Mode::Words(target) => {
                if generated_count < *target {
                    let remaining = *target - generated_count;
                    let mut new_words = word_controller::generate_next_word(&self.source, &self.rules, &context_strings, &mut rng);
                    formatting::apply_contextual_capitalization(&mut new_words, &context_strings, self.rules.use_punctuation);
                    // a word+dash pair could overshoot the last slot cap to remaining
                    new_words.truncate(remaining);
                    Some(new_words)
                } else {
                    None
                }
            }
        };

        new_raw_words.map(|strs| {
            let mut current_index = next_index;
            let words: Vec<Word> = strs.into_iter().map(|text| {
                let w = Word::new(text, current_index);
                current_index += 1;
                w
            }).collect();

            let new_next_index = current_index;
            (words, new_next_index)
        })
    }
}
