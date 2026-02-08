use super::punctuation::PunctuationRules;
use super::sourcing::TextSource;
use rand::Rng;

pub fn generate_time_batch(
    source: &TextSource,
    rules: &PunctuationRules,
    rng: &mut impl Rng
) -> Vec<String> {
    let mut stream = Vec::new();
    for _ in 0..100 {
        stream.extend(generate_smart_word(source, rules, rng));
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
    let mut stream = Vec::new();

    let raw_words = source.get_unique_batch(limit, rng);
    let generated_count = raw_words.len();

    for word in raw_words {
        stream.push(rules.apply(word, rng));
    }

    (stream, generated_count)
}

pub fn generate_next_word(
    source: &TextSource,
    rules: &PunctuationRules,
    existing_stream: &[String],
    rng: &mut impl Rng,
) -> Vec<String> {
    let mut raw_word = source.get_random_word(rng);

    if let Some(last) = existing_stream.last() {
        if last == &raw_word || last.contains(&raw_word) {
            raw_word = source.get_random_word(rng);
        }
    }

    let processed = rules.apply(raw_word, rng);
    vec![processed]
}

pub fn generate_smart_word(
    source: &TextSource,
    rules: &PunctuationRules,
    rng: &mut impl Rng,
) -> Vec<String> {
    let raw = source.get_random_word(rng);
    let processed = rules.apply(raw, rng);

    if rules.should_insert_dash(rng) {
        vec![processed, "-".to_string()]
    } else {
        vec![processed]
    }
}
