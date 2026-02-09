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
                let (stream, count) = word_controller::generate_count_batch(&self.source, &self.rules, *count, &mut rng);
                generated_count = count;
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

        let word_stream: Vec<Word> = raw_stream
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                let mut w = Word::new(text);
                if i == 0 {
                    w.state = WordState::Active;
                }
                w
            })
            .collect();

        GeneratedWords {
            word_stream,
            quote_pool,
            total_quote_words,
            current_quote_source,
            generated_count,
        }
    }

    pub fn add_one_word(
        &self,
        mode: &Mode,
        existing_stream: &[Word],
        quote_pool: &mut Vec<String>,
        generated_count: usize,
    ) -> Option<Vec<Word>> {
        let mut rng = rand::rng();

        let context_strings: Vec<String> = existing_stream.iter().map(|w| w.text.clone()).collect();

        let new_raw_words = match mode {
            Mode::Time(_) => {
                let mut new_words = word_controller::generate_smart_word(&self.source, &self.rules, &mut rng);
                formatting::apply_contextual_capitalization(&mut new_words, &context_strings, self.rules.use_punctuation);
                Some(new_words)
            }
            Mode::Quote(_) => {
                quote_controller::next_word(quote_pool)
            },
            Mode::Words(target) => {
                if generated_count < *target {
                    let mut new_words = word_controller::generate_next_word(&self.source, &self.rules, &context_strings, &mut rng);
                    formatting::apply_contextual_capitalization(&mut new_words, &context_strings, self.rules.use_punctuation);
                    Some(new_words)
                } else {
                    None
                }
            }
        };

        new_raw_words.map(|strs| {
            strs.into_iter().map(Word::new).collect()
        })
    }
}
