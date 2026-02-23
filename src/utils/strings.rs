pub fn are_characters_visually_equal(c1: char, c2: char) -> bool {
    c1 == c2
        || (is_quote(c1) && is_quote(c2))
        || (is_dash(c1) && is_dash(c2))
        || (is_comma_like(c1) && is_comma_like(c2))
}

pub fn clean_typography_symbols(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' | '\u{201C}' | '\u{201E}' => output.push('"'),
            '\u{2019}' | '\u{2018}' | '\u{1FBD}' | '\u{02BC}' => output.push('\''),
            '\u{2010}' => output.push('-'),
            '\u{00A0}' | '\u{2007}' | '\u{202F}' => output.push(' '),
            '\u{2026}' => output.push_str("..."),
            '\u{00AB}' => output.push_str("<<"),
            '\u{00BB}' => output.push_str(">>"),
            _ => output.push(c),
        }
    }
    output
}

pub fn capitalize_word(w: &mut String) {
    if let Some(idx) = w.find(|c: char| c.is_alphabetic()) {
        let mut chars: Vec<char> = w.chars().collect();
        if let Some(c) = chars.get_mut(idx) {
            *c = c.to_uppercase().next().unwrap_or(*c);
        }
        *w = chars.into_iter().collect();
    }
}

pub fn ends_with_terminator(w: &str) -> bool {
    w.ends_with('.') || w.ends_with('!') || w.ends_with('?')
}

pub fn is_sentence_end(w: &str) -> bool {
    !w.ends_with("...") && (w.ends_with('.') || w.ends_with('!') || w.ends_with('?'))
}

fn is_quote(c: char) -> bool {
    match c {
        '"' | '\u{201C}' | '\u{201D}' | '\u{201E}'
        | '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}' | '\u{1FBD}' => true,
        _ => false,
    }
}

fn is_dash(c: char) -> bool {
    match c {
        '-' | '\u{2013}' | '\u{2014}' | '\u{2010}' => true,
        _ => false,
    }
}

fn is_comma_like(c: char) -> bool {
    match c {
        ',' | '\u{201A}' => true,
        _ => false,
    }
}
