use std::collections::HashMap;
use std::sync::OnceLock;

static ABBREVIATIONS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn get_abbreviations() -> &'static HashMap<&'static str, &'static str> {
    ABBREVIATIONS.get_or_init(|| {
        const ABBREV_DATA: &str = include_str!("abbreviations.txt");

        ABBREV_DATA
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }
                let mut parts = line.split('=');
                let abbrev = parts.next()?.trim();
                let expansion = parts.next()?.trim();
                Some((abbrev, expansion))
            })
            .collect()
    })
}

pub fn expand_abbreviations(text: &str) -> String {
    let abbrev_map = get_abbreviations();

    let mut result = String::with_capacity(text.len() + 50);
    let chars = text.chars();
    let mut current_word = String::new();

    for ch in chars {
        if ch.is_alphabetic() || ch == '.' {
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                if let Some(expansion) = abbrev_map.get(current_word.as_str()) {
                    result.push_str(expansion);
                } else {
                    result.push_str(&current_word);
                }
                current_word.clear();
            }
            result.push(ch);
        }
    }

    // Handle last word if string doesn't end with delimiter
    if !current_word.is_empty() {
        if let Some(expansion) = abbrev_map.get(current_word.as_str()) {
            result.push_str(expansion);
        } else {
            result.push_str(&current_word);
        }
    }

    result
}
