use crate::utils::strings;

pub fn apply_contextual_capitalization(
    new_words: &mut [String],
    existing_stream: &[String],
    use_punctuation: bool,
) {
    if !use_punctuation { return; }
    if let Some(first_new) = new_words.first_mut() {
        if let Some(last_existing) = existing_stream.last() {
            if strings::is_sentence_end(last_existing) {
                strings::capitalize_word(first_new);
            }
        }
    }
}

pub fn finalize_stream_punctuation(stream: &mut Vec<String>) {
    if stream.is_empty() { return; }

    if let Some(first) = stream.first_mut() {
        strings::capitalize_word(first);
    }

    let len = stream.len();
    for i in 0..len - 1 {
        if strings::is_sentence_end(&stream[i]) {
            strings::capitalize_word(&mut stream[i + 1]);
        }
    }

    // remove any em dash that appears after punctuation that can't precede one
    // (sentence terminators, commas, semicolons, colons, opening parens, or another dash)
    let mut i = 1;
    while i < stream.len() {
        if stream[i] == "—" {
            let prev_last = stream[i - 1].chars().last();
            let bad_predecessor = stream[i - 1] == "—"
                || matches!(prev_last, Some('.' | '!' | '?' | ',' | ';' | ':' | '('));
            if bad_predecessor {
                stream.remove(i);
                continue;
            }
        }
        i += 1;
    }

    if let Some(last) = stream.last_mut() {
        // a trailing dash means the sentence was cut short, remove it
        if last == "-" || last == "—" {
            *last = String::new();
        }
        // strip punctuation that can't legally end a sentence
        if let Some(c) = last.chars().last() {
            if matches!(c, ',' | ';' | ':') {
                last.pop();
            }
        }
        // ellipsis is a valid ending, only force a period if truly bare
        let c = last.chars().last().unwrap_or(' ');
        if !matches!(c, '.' | '!' | '?') && !last.is_empty() {
            last.push('.');
        }
    }

    stream.retain(|s| !s.is_empty());
}
