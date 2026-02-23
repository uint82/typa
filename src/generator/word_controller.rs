use super::punctuation::{GenerationContext, PunctuationRules};
use super::sourcing::TextSource;
use crate::utils::strings;
use rand::Rng;

pub fn is_sentence_end_pub(word: &str) -> bool { strings::is_sentence_end(word) }

pub fn build_context_pub(stream: &[String]) -> GenerationContext { build_context(stream) }

fn build_context(stream: &[String]) -> GenerationContext {
    let mut ctx = GenerationContext::new();
    // replay the stream to get accurate counters without extra storage
    for word in stream {
        ctx.advance(word);
    }
    ctx
}

// em dash rules. these contexts must never precede an em dash:
// - sentence terminators (.!?) - a new sentence can't open with a dash
// - commas, semicolons, colons - pauses already cover the same breath-break role
// - opening parenthesis - dash inside parens looks wrong; paren does the same job
// - another em dash - two dashes in a row is never correct
// - empty string - em dash can never be the first token
fn can_precede_dash(word: &str) -> bool {
    word != "—"
        && !matches!(
            word.chars().last(),
            None | Some('.' | '!' | '?' | ',' | ';' | ':' | '(')
        )
}

// returns vec![word] normally, or vec![word, "—"] when grammatically valid at ~4%
fn maybe_append_dash(word: String, rules: &PunctuationRules, rng: &mut impl Rng) -> Vec<String> {
    if can_precede_dash(&word) && rules.should_insert_dash(rng) {
        vec![word, "—".to_string()]
    } else {
        vec![word]
    }
}

pub fn generate_time_batch(
    source: &TextSource,
    rules: &PunctuationRules,
    rng: &mut impl Rng,
) -> Vec<String> {
    let mut stream: Vec<String> = Vec::new();
    let mut ctx = GenerationContext::new();
    for _ in 0..100 {
        let is_sentence_start = stream.last().map(|w| strings::is_sentence_end(w)).unwrap_or(true);
        let new_words = generate_smart_word(source, rules, rng, is_sentence_start, &ctx);
        for w in &new_words { ctx.advance(w); }
        stream.extend(new_words);
    }
    stream
}

pub fn generate_count_batch(
    source: &TextSource,
    rules: &PunctuationRules,
    count: usize,
    rng: &mut impl Rng,
) -> (Vec<String>, usize) {
    let limit = count.min(100);
    let mut stream: Vec<String> = Vec::new();
    let mut ctx = GenerationContext::new();

    let raw_words = source.get_unique_batch(limit, rng);

    for word in raw_words {
        let is_sentence_start = stream.last().map(|w| strings::is_sentence_end(w)).unwrap_or(true);
        let placed = rules.apply(word, rng, is_sentence_start, &ctx);
        let new_words = maybe_append_dash(placed, rules, rng);
        for w in &new_words { ctx.advance(w); }
        stream.extend(new_words);
    }

    // em dashes can push the stream past the requested count
    // cap it exactly
    stream.truncate(count);

    let generated_count = stream.len();
    (stream, generated_count)
}

pub fn generate_next_word(
    source: &TextSource,
    rules: &PunctuationRules,
    existing_stream: &[String],
    rng: &mut impl Rng,
) -> Vec<String> {
    // strip punctuation before comparing so "fast," doesn't pass "fast" through the dedup check
    let recent: Vec<String> = existing_stream.iter().rev().take(8)
        .map(|w| w.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\'').to_string())
        .collect();

    let mut raw_word = source.get_random_word(rng);
    for _ in 0..2 {
        if recent.iter().any(|w| w == &raw_word) {
            raw_word = source.get_random_word(rng);
        } else {
            break;
        }
    }

    let is_sentence_start = existing_stream.last().map(|w| strings::is_sentence_end(w)).unwrap_or(true);
    let ctx = build_context(existing_stream);
    let placed = rules.apply(raw_word, rng, is_sentence_start, &ctx);
    maybe_append_dash(placed, rules, rng)
}

pub fn generate_smart_word(
    source: &TextSource,
    rules: &PunctuationRules,
    rng: &mut impl Rng,
    is_sentence_start: bool,
    ctx: &GenerationContext,
) -> Vec<String> {
    let raw = source.get_random_word(rng);
    let processed = rules.apply(raw, rng, is_sentence_start, ctx);
    maybe_append_dash(processed, rules, rng)
}
