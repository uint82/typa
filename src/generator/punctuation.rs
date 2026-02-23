use rand::prelude::IndexedRandom;
use rand::Rng;
use crate::utils::strings;

const MIN_SENTENCE_WORDS: usize = 6;
const MIN_COMMA_GAP: usize = 3;

pub struct PunctuationRules {
    pub use_punctuation: bool,
    pub use_numbers: bool,
}

/// state the caller threads through so apply() can make gap-aware decisions
pub struct GenerationContext {
    pub words_since_terminator: usize,
    pub words_since_last_comma: usize,
}

impl GenerationContext {
    pub fn new() -> Self {
        Self { words_since_terminator: 0, words_since_last_comma: MIN_COMMA_GAP }
    }

    /// call after every word is placed to advance the counters
    pub fn advance(&mut self, placed_word: &str) {
        if strings::is_sentence_end(placed_word) {
            self.words_since_terminator = 0;
            self.words_since_last_comma = MIN_COMMA_GAP;
        } else {
            self.words_since_terminator += 1;
            if placed_word.ends_with(',') {
                self.words_since_last_comma = 0;
            } else {
                self.words_since_last_comma += 1;
            }
        }
    }
}

impl PunctuationRules {
    pub fn apply(
        &self,
        mut word: String,
        rng: &mut impl Rng,
        is_sentence_start: bool,
        ctx: &GenerationContext,
    ) -> String {
        // digits look wrong at sentence start (right after . ! ?)
        if self.use_numbers && !is_sentence_start && rng.random_bool(0.12) {
            return self.generate_number(rng);
        }

        if !self.use_punctuation {
            return word;
        }

        if rng.random_bool(0.35) {
            word = self.apply_contraction(&word, rng);
        }

        // ~20% of words carry punctuation tuned to resemble natural English prose density
        if rng.random_bool(0.20) {
            let can_end_sentence = ctx.words_since_terminator >= MIN_SENTENCE_WORDS;
            let can_comma = ctx.words_since_last_comma >= MIN_COMMA_GAP;

            let p_type = rng.random_range(0..100u32);
            match p_type {
                // comma: 25% share (down from 40%) and gated by MIN_COMMA_GAP
                0..=24 => {
                    if can_comma { word.push(','); }
                }
                25..=42 => {
                    if can_end_sentence { word.push('.'); }
                }
                43..=52 => {
                    if can_end_sentence { word.push(';'); }
                }
                53..=57 => {
                    if can_end_sentence { word.push(':'); }
                }
                58..=65 => {
                    if can_end_sentence { word.push('!'); }
                }
                66..=73 => {
                    if can_end_sentence { word.push('?'); }
                }
                74..=78 => {
                    // ellipsis is fine at any point. it trails off rather than ends
                    word.push_str("...");
                }
                79..=89 => word = format!("\"{}\"", word),
                90..=99 => word = format!("({})", word),
                _       => {}
            }
        }
        word
    }

    // em-dash appears occasionally to interrupt or join clauses
    pub fn should_insert_dash(&self, rng: &mut impl Rng) -> bool {
        self.use_punctuation && rng.random_bool(0.02)
    }

    fn generate_number(&self, rng: &mut impl Rng) -> String {
        match rng.random_range(0..100u32) {
            0..=34  => rng.random_range(0..=9999u32).to_string(),
            35..=54 => {
                let n = rng.random_range(1..=100u32);
                format!("{}{}", n, ordinal_suffix(n))
            }
            55..=69 => {
                let whole = rng.random_range(0..=99u32);
                let frac  = rng.random_range(0..=9u32);
                format!("{}.{}", whole, frac)
            }
            70..=79 => format!("{}%", rng.random_range(1..=100u32)),
            80..=89 => format!("-{}", rng.random_range(1..=999u32)),
            // en dash ranges: years, pages, scores, quantities
            90..=99 => {
                let lo = rng.random_range(1..=999u32);
                let hi = lo + rng.random_range(1..=100u32);
                format!("{}–{}", lo, hi)
            }
            _       => rng.random_range(0..=9999u32).to_string(),
        }
    }

    fn apply_contraction(&self, original: &str, rng: &mut impl Rng) -> String {
        let lower = original.to_lowercase();
        if let Some(replacements) = self.get_contraction_replacements(&lower) {
            if let Some(replacement) = replacements.choose(rng) {
                return self.match_casing(original, replacement);
            }
        }
        original.to_string()
    }

    fn match_casing(&self, original: &str, replacement: &str) -> String {
        // require more than one char. a lone "I" is not an acronym and should not fully uppercase
        let is_all_upper = original.chars().count() > 1
            && original.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());

        if is_all_upper {
            return replacement.to_uppercase();
        }

        let first_is_upper = original.chars().next().map_or(false, |c| c.is_uppercase());
        if first_is_upper {
            let mut chars = replacement.chars();
            chars.next().map_or_else(String::new, |f| {
                f.to_uppercase().collect::<String>() + chars.as_str()
            })
        } else {
            replacement.to_string()
        }
    }

    fn get_contraction_replacements(&self, word: &str) -> Option<&'static [&'static str]> {
        match word {
            "are"    => Some(&["aren't"]),
            "can"    => Some(&["can't"]),
            "cannot" => Some(&["can't"]),
            "could"  => Some(&["couldn't"]),
            "did"    => Some(&["didn't"]),
            "does"   => Some(&["doesn't"]),
            "do"     => Some(&["don't"]),
            "had"    => Some(&["hadn't"]),
            "has"    => Some(&["hasn't"]),
            "have"   => Some(&["haven't"]),
            "is"     => Some(&["isn't"]),
            "it"     => Some(&["it's", "it'll"]),
            "i"      => Some(&["i'm", "i'll", "i've", "i'd"]),
            "you"    => Some(&["you'll", "you're", "you've", "you'd"]),
            "that"   => Some(&["that's", "that'll", "that'd"]),
            "must"   => Some(&["mustn't", "must've"]),
            "there"  => Some(&["there's", "there'll", "there'd"]),
            "he"     => Some(&["he's", "he'll", "he'd"]),
            "she"    => Some(&["she's", "she'll", "she'd"]),
            "we"     => Some(&["we're", "we'll", "we'd", "we've"]),
            "they"   => Some(&["they're", "they'll", "they'd", "they've"]),
            "should" => Some(&["shouldn't", "should've"]),
            "was"    => Some(&["wasn't"]),
            "were"   => Some(&["weren't"]),
            "will"   => Some(&["won't"]),
            "would"  => Some(&["wouldn't", "would've"]),
            "let"    => Some(&["let's"]),
            "what"   => Some(&["what's"]),
            "who"    => Some(&["who's"]),
            "where"  => Some(&["where's"]),
            "how"    => Some(&["how's"]),
            "ain"    => Some(&["ain't"]),
            "going"  => Some(&["gonna", "goin'"]),
            "got"    => Some(&["gotta"]),
            "want"   => Some(&["wanna"]),
            _        => None,
        }
    }
}

fn ordinal_suffix(n: u32) -> &'static str {
    // teens (11th–13th) are irregular. they always use "th"
    match n % 100 {
        11 | 12 | 13 => "th",
        _ => match n % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    }
}
