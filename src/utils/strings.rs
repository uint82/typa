pub fn are_characters_visually_equal(c1: char, c2: char) -> bool {
    if c1 == c2 {
        return true;
    }

    if is_quote(c1) && is_quote(c2) {
        return true;
    }
    if is_dash(c1) && is_dash(c2) {
        return true;
    }
    if is_comma_like(c1) && is_comma_like(c2) {
        return true;
    }

    false
}

pub fn clean_typography_symbols(text: &str) -> String {
    let mut output = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '“' | '”' | '„' => output.push('"'),
            '’' | '‘' | '᾽' | 'ʼ' => output.push('\''),

            '—' | '–' | '‐' => output.push('-'),

            '\u{00A0}' | '\u{2007}' | '\u{202F}' => output.push(' '),

            '…' => output.push_str("..."),
            '«' => output.push_str("<<"),
            '»' => output.push_str(">>"),

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
    w.contains('.') || w.contains('!') || w.contains('?')
}

fn is_quote(c: char) -> bool {
    matches!(c, '"' | '“' | '”' | '„' | '\'' | '’' | '‘' | 'ʼ' | '᾽')
}

fn is_dash(c: char) -> bool {
    matches!(c, '-' | '–' | '—' | '‐')
}

fn is_comma_like(c: char) -> bool {
    matches!(c, ',' | '‚')
}
