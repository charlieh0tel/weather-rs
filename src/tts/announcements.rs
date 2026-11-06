use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    MetarData, WeatherGroup, celsius_to_fahrenheit, expand_station_name, parse_weather_groups,
};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum AnnouncementFormat {
    /// Direct speech output
    Speech,
    /// Brief announcement format
    Brief,
    /// Detailed weather report
    Detailed,
    /// Aviation radio style
    Aviation,
}

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
const STALE_AFTER_MINUTES: i64 = 90;

fn spell_out_icao(icao: &str) -> String {
    icao.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn spoken_digits(number: i64, width: usize) -> String {
    const DIGITS: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    format!("{number:0width$}")
        .bytes()
        .map(|digit| DIGITS[usize::from(digit - b'0')])
        .collect::<Vec<_>>()
        .join(" ")
}

fn spoken_observation(obs_time: Option<i64>, now: i64) -> String {
    let Some(obs_time) = obs_time else {
        return "Observation time not available... ".to_string();
    };

    let minutes_of_day = obs_time.rem_euclid(SECONDS_PER_DAY) / 60;
    let time = minutes_of_day / 60 * 100 + minutes_of_day % 60;
    let mut spoken = format!("Observed {} zulu... ", spoken_digits(time, 4));

    let age_minutes = (now - obs_time) / 60;
    if age_minutes > STALE_AFTER_MINUTES {
        spoken.push_str(&format!(
            "Warning... this report is {age_minutes} minutes old... "
        ));
    }
    spoken
}

fn spoken_group(group: &WeatherGroup) -> &str {
    group
        .description
        .as_deref()
        .unwrap_or("Unrecognized weather")
}

fn spoken_degrees(degrees: f64) -> String {
    let rounded = degrees.round() as i32;
    if rounded < 0 {
        format!("minus {}", rounded.unsigned_abs())
    } else {
        rounded.to_string()
    }
}

pub fn generate_weather_announcement(metar: &MetarData, format: &AnnouncementFormat) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    announcement_at(metar, format, now)
}

fn announcement_at(metar: &MetarData, format: &AnnouncementFormat, now: i64) -> String {
    let observation = spoken_observation(metar.obs_time, now);
    let groups = metar
        .wx_string
        .as_deref()
        .map(parse_weather_groups)
        .unwrap_or_default();

    match format {
        AnnouncementFormat::Speech | AnnouncementFormat::Brief => {
            let mut announcement = format!("Weather for {}... ", spell_out_icao(&metar.icao_id));

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_station_name(name)));
            }

            announcement.push_str(&observation);

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... ",
                    spoken_degrees(temp_f)
                ));
            }

            if groups.is_empty() {
                announcement.push_str("No significant weather reported...");
            } else {
                announcement.push_str("Conditions... ");
                let conditions: Vec<&str> = groups.iter().map(spoken_group).collect();
                announcement.push_str(&conditions.join("... "));
                announcement.push_str("...");
            }

            announcement
        }

        AnnouncementFormat::Detailed => {
            let mut announcement = format!(
                "Detailed weather report for {}... ",
                spell_out_icao(&metar.icao_id)
            );

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_station_name(name)));
            }

            announcement.push_str(&format!("Raw METAR... {}... ", metar.raw_ob));
            announcement.push_str(&observation);

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... {} degrees celsius... ",
                    spoken_degrees(temp_f),
                    spoken_degrees(temp_c)
                ));
            } else {
                announcement.push_str("Temperature... not available... ");
            }

            if let Some(wx) = metar.wx_string.as_deref().filter(|_| !groups.is_empty()) {
                announcement.push_str(&format!("Weather string... {}... ", wx));
                announcement.push_str("Weather codes found... ");
                let code_descriptions: Vec<String> = groups
                    .iter()
                    .map(|group| format!("{} ({})", spoken_group(group), group.raw))
                    .collect();
                announcement.push_str(&code_descriptions.join("... "));
                announcement.push_str("...");
            } else {
                announcement.push_str("Weather... none reported... No weather codes found...");
            }

            announcement
        }

        AnnouncementFormat::Aviation => {
            let mut announcement = format!("{} weather... ", spell_out_icao(&metar.icao_id));
            announcement.push_str(&observation);

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature {} degrees... ",
                    spoken_degrees(temp_f)
                ));
            }

            if groups.is_empty() {
                announcement.push_str("No significant weather reported... ");
            } else {
                for group in &groups {
                    announcement.push_str(&format!("{}... ", spoken_group(group)));
                }
            }

            announcement.push_str("End weather...");
            announcement
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_OB: &str = "METAR KSFO 061256Z -RA BR";
    // 16:28 UTC
    const OBSERVED_AT: i64 = 20_000 * SECONDS_PER_DAY + 16 * 3600 + 28 * 60;

    fn announce(metar: &MetarData, format: &AnnouncementFormat) -> String {
        announcement_at(metar, format, OBSERVED_AT + 600)
    }

    fn bare_metar() -> MetarData {
        MetarData {
            icao_id: "KSFO".to_string(),
            raw_ob: RAW_OB.to_string(),
            obs_time: Some(OBSERVED_AT),
            temp: None,
            wx_string: None,
            name: None,
        }
    }

    fn full_metar() -> MetarData {
        MetarData {
            temp: Some(20.0),
            wx_string: Some("-RA BR".to_string()),
            name: Some("San Francisco Intl, CA, US".to_string()),
            ..bare_metar()
        }
    }

    fn unrecognized_wx_metar() -> MetarData {
        MetarData {
            wx_string: Some("XX".to_string()),
            ..bare_metar()
        }
    }

    #[test]
    fn speech_and_brief_announcements() {
        let cases = [
            (
                full_metar(),
                "Weather for K S F O... San Francisco International, California, U S... Observed one six two eight zulu... Temperature... 68 degrees fahrenheit... Conditions... Light rain... Mist...",
            ),
            (
                bare_metar(),
                "Weather for K S F O... Observed one six two eight zulu... No significant weather reported...",
            ),
            (
                unrecognized_wx_metar(),
                "Weather for K S F O... Observed one six two eight zulu... Conditions... Unrecognized weather...",
            ),
        ];

        for (metar, expected) in cases {
            for format in [AnnouncementFormat::Speech, AnnouncementFormat::Brief] {
                assert_eq!(announce(&metar, &format), expected);
            }
        }
    }

    #[test]
    fn detailed_announcements() {
        let cases = [
            (
                full_metar(),
                format!(
                    "Detailed weather report for K S F O... San Francisco International, California, U S... Raw METAR... {RAW_OB}... Observed one six two eight zulu... Temperature... 68 degrees fahrenheit... 20 degrees celsius... Weather string... -RA BR... Weather codes found... Light rain (-RA)... Mist (BR)..."
                ),
            ),
            (
                bare_metar(),
                format!(
                    "Detailed weather report for K S F O... Raw METAR... {RAW_OB}... Observed one six two eight zulu... Temperature... not available... Weather... none reported... No weather codes found..."
                ),
            ),
            (
                unrecognized_wx_metar(),
                format!(
                    "Detailed weather report for K S F O... Raw METAR... {RAW_OB}... Observed one six two eight zulu... Temperature... not available... Weather string... XX... Weather codes found... Unrecognized weather (XX)..."
                ),
            ),
        ];

        for (metar, expected) in cases {
            assert_eq!(announce(&metar, &AnnouncementFormat::Detailed), expected);
        }
    }

    #[test]
    fn aviation_announcements() {
        let cases = [
            (
                full_metar(),
                "K S F O weather... Observed one six two eight zulu... Temperature 68 degrees... Light rain... Mist... End weather...",
            ),
            (
                bare_metar(),
                "K S F O weather... Observed one six two eight zulu... No significant weather reported... End weather...",
            ),
            (
                unrecognized_wx_metar(),
                "K S F O weather... Observed one six two eight zulu... Unrecognized weather... End weather...",
            ),
        ];

        for (metar, expected) in cases {
            assert_eq!(announce(&metar, &AnnouncementFormat::Aviation), expected);
        }
    }

    #[test]
    fn unknown_precipitation_is_announced() {
        let metar = MetarData {
            wx_string: Some("UP".to_string()),
            ..bare_metar()
        };

        assert_eq!(
            announce(&metar, &AnnouncementFormat::Speech),
            "Weather for K S F O... Observed one six two eight zulu... Conditions... Unknown precipitation..."
        );
        assert_eq!(
            announce(&metar, &AnnouncementFormat::Aviation),
            "K S F O weather... Observed one six two eight zulu... Unknown precipitation... End weather..."
        );
    }

    #[test]
    fn no_significant_weather_is_announced_like_no_weather() {
        let metar = MetarData {
            wx_string: Some("NSW".to_string()),
            ..bare_metar()
        };

        for format in [
            AnnouncementFormat::Speech,
            AnnouncementFormat::Brief,
            AnnouncementFormat::Detailed,
            AnnouncementFormat::Aviation,
        ] {
            assert_eq!(announce(&metar, &format), announce(&bare_metar(), &format));
        }
    }

    #[test]
    fn hazard_qualifiers_are_spoken_in_report_order() {
        let metar = MetarData {
            wx_string: Some("VCTS -FZRA".to_string()),
            ..bare_metar()
        };

        assert_eq!(
            announce(&metar, &AnnouncementFormat::Aviation),
            "K S F O weather... Observed one six two eight zulu... Thunderstorm in the vicinity... Light freezing rain... End weather..."
        );
    }

    #[test]
    fn speaks_the_observation_time_with_leading_zeros() {
        let cases = [
            (0, "Observed zero zero zero zero zulu... "),
            (9 * 60 + 5, "Observed zero nine zero five zulu... "),
            (23 * 60 + 59, "Observed two three five nine zulu... "),
        ];

        for (minutes, expected) in cases {
            let obs_time = 20_000 * SECONDS_PER_DAY + minutes * 60;
            assert_eq!(spoken_observation(Some(obs_time), obs_time), expected);
        }
    }

    #[test]
    fn warns_only_about_old_reports() {
        let warned = |minutes: i64| {
            spoken_observation(Some(OBSERVED_AT), OBSERVED_AT + minutes * 60).contains("Warning")
        };

        assert!(!warned(0));
        assert!(!warned(90));
        assert!(warned(91));
        assert!(
            spoken_observation(Some(OBSERVED_AT), OBSERVED_AT + 125 * 60)
                .contains("this report is 125 minutes old")
        );
    }

    #[test]
    fn says_when_the_observation_time_is_missing() {
        let metar = MetarData {
            obs_time: None,
            ..bare_metar()
        };

        assert!(
            announce(&metar, &AnnouncementFormat::Aviation)
                .contains("Observation time not available")
        );
    }

    #[test]
    fn temperatures_are_rounded_to_whole_degrees() {
        let metar = MetarData {
            temp: Some(21.5),
            ..bare_metar()
        };

        let announcement = announce(&metar, &AnnouncementFormat::Detailed);

        assert!(
            announcement.contains("71 degrees fahrenheit... 22 degrees celsius"),
            "{announcement}"
        );
    }

    #[test]
    fn negative_temperatures_are_spoken_as_minus() {
        let metar = MetarData {
            temp: Some(-40.0),
            ..bare_metar()
        };

        let cases = [
            (AnnouncementFormat::Speech, "minus 40 degrees fahrenheit"),
            (AnnouncementFormat::Brief, "minus 40 degrees fahrenheit"),
            (
                AnnouncementFormat::Detailed,
                "minus 40 degrees fahrenheit... minus 40 degrees celsius",
            ),
            (AnnouncementFormat::Aviation, "Temperature minus 40 degrees"),
        ];

        for (format, expected) in cases {
            let announcement = announce(&metar, &format);
            assert!(announcement.contains(expected), "{announcement}");
            assert!(!announcement.contains("-40"), "{announcement}");
        }
    }

    #[test]
    fn temperatures_rounding_to_zero_are_not_negative() {
        let metar = MetarData {
            temp: Some(-0.4),
            ..bare_metar()
        };

        let announcement = announce(&metar, &AnnouncementFormat::Detailed);

        assert!(
            announcement.contains("31 degrees fahrenheit... 0 degrees celsius"),
            "{announcement}"
        );
    }
}
