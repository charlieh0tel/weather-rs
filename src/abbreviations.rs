use std::collections::HashMap;
use std::sync::OnceLock;

const ABBREV_DATA: &str = include_str!("abbreviations.txt");

static ABBREVIATIONS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn get_abbreviations() -> &'static HashMap<&'static str, &'static str> {
    ABBREVIATIONS.get_or_init(|| {
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

fn push_expanded(result: &mut String, word: &str) {
    let abbrev_map = get_abbreviations();

    if let Some(expansion) = abbrev_map.get(word) {
        result.push_str(expansion);
        return;
    }

    let stem = word.trim_end_matches('.');
    if let Some(expansion) = abbrev_map.get(stem) {
        result.push_str(expansion);
        result.push_str(&word[stem.len()..]);
    } else {
        result.push_str(word);
    }
}

pub fn expand_abbreviations(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 50);
    let mut current_word = String::new();

    for ch in text.chars() {
        if ch.is_alphabetic() || ch == '.' {
            current_word.push(ch);
        } else {
            push_expanded(&mut result, &current_word);
            current_word.clear();
            result.push(ch);
        }
    }

    push_expanded(&mut result, &current_word);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn expands_abbreviations() {
        let cases = [
            ("", ""),
            ("Intl", "International"),
            ("San Jose Arpt", "San Jose Airport"),
            ("Intl Arpt", "International Airport"),
            ("San Jose Intl, CA", "San Jose International, California"),
            ("St. Louis", "Saint Louis"),
            ("Ft. Worth, TX", "Fort Worth, Texas"),
            ("Arpt2", "Airport2"),
            ("Zürich Intl", "Zürich International"),
            ("Intl.", "International."),
            ("Near San Jose, CA.", "Near San Jose, California."),
            ("Arpt...", "Airport..."),
        ];

        for (input, expected) in cases {
            assert_eq!(expand_abbreviations(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn leaves_non_matching_words_unchanged() {
        let cases = [
            "Unknown Field",
            "intl",
            "St",
            "U.S.",
            "...",
            "Mt.Hood",
            "Runway 12",
        ];

        for input in cases {
            assert_eq!(expand_abbreviations(input), input);
        }
    }

    #[test]
    fn abbreviation_data_is_well_formed() {
        let mut seen = HashSet::new();

        for line in ABBREV_DATA
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let (abbrev, expansion) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("missing '=': {line:?}"));
            let (abbrev, expansion) = (abbrev.trim(), expansion.trim());

            assert!(!expansion.is_empty(), "empty expansion: {line:?}");
            assert!(!expansion.contains('='), "extra '=': {line:?}");
            assert!(
                !abbrev.is_empty() && abbrev.chars().all(|c| c.is_alphabetic() || c == '.'),
                "abbreviation can never match a word: {line:?}"
            );
            assert!(seen.insert(abbrev), "duplicate abbreviation: {line:?}");
        }
    }
}
