use crate::WmoCode;

/// One whitespace-separated group of a METAR weather string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherGroup {
    pub raw: String,
    /// What the group means in words, or `None` if it is not understood.
    pub description: Option<String>,
}

/// Parses a METAR weather string, keeping the order of the report.
///
/// `NSW` ("no significant weather") reports an absence of weather, so it yields no group.
pub fn parse_weather_groups(wx_string: &str) -> Vec<WeatherGroup> {
    wx_string
        .split_whitespace()
        .filter(|group| *group != "NSW")
        .map(|group| WeatherGroup {
            raw: group.to_string(),
            description: describe(group),
        })
        .collect()
}

fn phenomenon_name(code: &WmoCode, heavy: bool) -> String {
    match code {
        WmoCode::FunnelCloud if heavy => "tornado or waterspout".to_string(),
        WmoCode::FunnelCloud => "funnel cloud".to_string(),
        _ => code.description().replace('/', " or ").to_lowercase(),
    }
}

fn describe(group: &str) -> Option<String> {
    let (intensity, rest) = match group.as_bytes().first()? {
        b'+' => (Some("heavy"), &group[1..]),
        b'-' => (Some("light"), &group[1..]),
        _ => (None, group),
    };
    let (vicinity, rest) = match rest.strip_prefix("VC") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let heavy = intensity == Some("heavy");

    let mut phenomena = Vec::new();
    let mut prefix = None;
    let (mut showers, mut thunderstorm) = (false, false);
    for pair in rest.as_bytes().chunks(2) {
        match pair {
            b"MI" => prefix = Some("shallow"),
            b"PR" => prefix = Some("partial"),
            b"BC" => prefix = Some("patches of"),
            b"DR" => prefix = Some("low drifting"),
            b"BL" => prefix = Some("blowing"),
            b"FZ" => prefix = Some("freezing"),
            b"SH" => showers = true,
            b"TS" => thunderstorm = true,
            _ => phenomena.push(phenomenon_name(
                &WmoCode::from_code(std::str::from_utf8(pair).ok()?)?,
                heavy,
            )),
        }
    }

    let phenomena = phenomena.join(" and ");
    let mut description = match (prefix, phenomena.is_empty()) {
        (Some(_), true) => return None,
        (Some(prefix), false) => format!("{prefix} {phenomena}"),
        (None, _) => phenomena,
    };
    if showers {
        description = if description.is_empty() {
            "showers".to_string()
        } else {
            format!("{description} showers")
        };
    }
    if thunderstorm {
        description = if description.is_empty() {
            "thunderstorm".to_string()
        } else {
            format!("thunderstorm with {description}")
        };
    }
    if description.is_empty() {
        return None;
    }

    // The tornado wording already conveys the intensity.
    if let Some(intensity) = intensity
        && !description.starts_with("tornado")
    {
        description = format!("{intensity} {description}");
    }
    if vicinity {
        description.push_str(" in the vicinity");
    }

    let mut chars = description.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptions(wx: &str) -> Vec<Option<String>> {
        parse_weather_groups(wx)
            .into_iter()
            .map(|group| group.description)
            .collect()
    }

    #[test]
    fn describes_weather_groups() {
        let cases = [
            ("RA", "Rain"),
            ("-RA", "Light rain"),
            ("+RA", "Heavy rain"),
            ("BR", "Mist"),
            ("FZRA", "Freezing rain"),
            ("-FZDZ", "Light freezing drizzle"),
            ("FZFG", "Freezing fog"),
            ("MIFG", "Shallow fog"),
            ("PRFG", "Partial fog"),
            ("BCFG", "Patches of fog"),
            ("BLSN", "Blowing snow"),
            ("DRSN", "Low drifting snow"),
            ("SHRA", "Rain showers"),
            ("-SHRASN", "Light rain and snow showers"),
            ("VCSH", "Showers in the vicinity"),
            ("TS", "Thunderstorm"),
            ("VCTS", "Thunderstorm in the vicinity"),
            ("TSRA", "Thunderstorm with rain"),
            ("+TSRA", "Heavy thunderstorm with rain"),
            ("-VCTSRA", "Light thunderstorm with rain in the vicinity"),
            ("TSSN", "Thunderstorm with snow"),
            ("TSGR", "Thunderstorm with hail"),
            ("TSGS", "Thunderstorm with small hail or snow pellets"),
            ("FC", "Funnel cloud"),
            ("+FC", "Tornado or waterspout"),
            ("PO", "Dust or sand whirls"),
            ("UP", "Unknown precipitation"),
            ("+SS", "Heavy sandstorm"),
        ];

        for (wx, expected) in cases {
            assert_eq!(
                descriptions(wx),
                [Some(expected.to_string())],
                "wx string: {wx:?}"
            );
        }
    }

    #[test]
    fn does_not_understand_malformed_groups() {
        for wx in ["XX", "R", "-", "FZ", "VC", "//", "RERA", "é"] {
            assert_eq!(descriptions(wx), [None], "wx string: {wx:?}");
        }
    }

    #[test]
    fn keeps_the_order_of_the_report() {
        let groups = parse_weather_groups("VCTS -RA BR");

        assert_eq!(
            groups
                .iter()
                .map(|group| group.raw.as_str())
                .collect::<Vec<_>>(),
            ["VCTS", "-RA", "BR"]
        );
    }

    #[test]
    fn no_significant_weather_and_empty_strings_have_no_groups() {
        for wx in ["", "  ", "NSW"] {
            assert!(parse_weather_groups(wx).is_empty(), "wx string: {wx:?}");
        }
    }
}
