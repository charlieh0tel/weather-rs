use std::collections::HashMap;
use std::sync::OnceLock;

type Table = HashMap<&'static str, &'static str>;

const ABBREV_DATA: &str = include_str!("abbreviations.txt");
const US_STATE_DATA: &str = include_str!("us_states.txt");

static ABBREVIATIONS: OnceLock<Table> = OnceLock::new();
static US_STATES: OnceLock<Table> = OnceLock::new();

fn parse_table(data: &'static str) -> Table {
    data.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (abbrev, expansion) = line.split_once('=')?;
            Some((abbrev.trim(), expansion.trim()))
        })
        .collect()
}

fn get_abbreviations() -> &'static Table {
    ABBREVIATIONS.get_or_init(|| parse_table(ABBREV_DATA))
}

fn get_us_states() -> &'static Table {
    US_STATES.get_or_init(|| parse_table(US_STATE_DATA))
}

fn push_expanded(result: &mut String, word: &str) {
    let abbrev_map = get_abbreviations();

    if let Some(expansion) = abbrev_map.get(word) {
        result.push_str(expansion);
        return;
    }

    let stem = word.trim_end_matches('.');
    let trailing_periods = &word[stem.len()..];

    if let Some(expansion) = abbrev_map.get(stem) {
        result.push_str(expansion);
        result.push_str(trailing_periods);
        return;
    }

    // Requiring two or more letters after the period leaves initials like "N.Y." alone.
    if let Some((prefix, rest)) = stem.split_once('.')
        && rest.chars().count() >= 2
        && !rest.contains('.')
        && let Some(expansion) = abbrev_map.get(&stem[..=prefix.len()])
    {
        result.push_str(expansion);
        result.push(' ');
        result.push_str(rest);
        result.push_str(trailing_periods);
        return;
    }

    result.push_str(word);
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

/// Makes a station name from the API (`site, state, country`) fit to speak.
///
/// State and country codes are only spoken for US stations, because elsewhere they read as US
/// states: "CA, CO" is Cundinamarca, Colombia.
pub fn expand_station_name(name: &str) -> String {
    let mut fields = name.rsplitn(3, ", ");
    let (Some(country), Some(state), Some(site)) = (fields.next(), fields.next(), fields.next())
    else {
        return expand_abbreviations(name);
    };

    let site = expand_abbreviations(site);
    if country != "US" {
        return site;
    }
    match get_us_states().get(state) {
        Some(state) => format!("{site}, {state}, U S"),
        None => format!("{site}, U S"),
    }
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
            ("San Jose Intl, CA", "San Jose International, CA"),
            ("St. Louis", "Saint Louis"),
            ("Ft. Worth, TX", "Fort Worth, TX"),
            ("Arpt2", "Airport2"),
            ("Zürich Intl", "Zürich International"),
            ("Intl.", "International."),
            ("Near San Jose Arpt.", "Near San Jose Airport."),
            ("Arpt...", "Airport..."),
            ("Mt.Hood", "Mount Hood"),
            ("St.Louis, MO", "Saint Louis, MO"),
            ("Ft.Worth.", "Fort Worth."),
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
            "Xx.Hood",
            "N.Y.",
            "N.Y",
            "S.F.O",
            "Runway 12",
        ];

        for input in cases {
            assert_eq!(expand_abbreviations(input), input);
        }
    }

    #[test]
    fn expands_station_names() {
        let cases = [
            (
                "San Jose Intl Arpt, CA, US",
                "San Jose International Airport, California, U S",
            ),
            (
                "Anchorage Intl, AK, US",
                "Anchorage International, Alaska, U S",
            ),
            ("Somewhere Arpt, , US", "Somewhere Airport, U S"),
            ("Bogota/El Dorado Arpt, CA, CO", "Bogota/El Dorado Airport"),
            ("Oslo/Gardermoen Arpt, AK, NO", "Oslo/Gardermoen Airport"),
            (
                "Mumbai/Shivaji Intl, MM, IN",
                "Mumbai/Shivaji International",
            ),
            ("San Juan/Marin Intl, , PR", "San Juan/Marin International"),
            ("No Fields Arpt", "No Fields Airport"),
            ("One, Comma", "One, Comma"),
        ];

        for (name, expected) in cases {
            assert_eq!(expand_station_name(name), expected, "name: {name:?}");
        }
    }

    #[test]
    fn abbreviation_data_is_well_formed() {
        for data in [ABBREV_DATA, US_STATE_DATA] {
            let mut seen = HashSet::new();

            for line in data
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
}
