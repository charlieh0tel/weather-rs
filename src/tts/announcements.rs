use crate::{MetarData, celsius_to_fahrenheit, expand_abbreviations, parse_wmo_codes};

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

fn spell_out_icao(icao: &str) -> String {
    icao.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
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
    // NSW ("no significant weather") reports an absence of weather, not an unrecognized code.
    let wx_string = metar.wx_string.as_deref().filter(|wx| wx.trim() != "NSW");

    match format {
        AnnouncementFormat::Speech | AnnouncementFormat::Brief => {
            let mut announcement = format!("Weather for {}... ", spell_out_icao(&metar.icao_id));

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature... {} degrees fahrenheit... ",
                    spoken_degrees(temp_f)
                ));
            }

            if let Some(wx) = wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Current conditions... ");
                    let conditions: Vec<String> =
                        codes.iter().map(|c| c.description().to_string()).collect();
                    announcement.push_str(&conditions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("Conditions not recognized...");
                }
            } else {
                announcement.push_str("No significant weather reported...");
            }

            announcement
        }

        AnnouncementFormat::Detailed => {
            let mut announcement = format!(
                "Detailed weather report for {}... ",
                spell_out_icao(&metar.icao_id)
            );

            if let Some(ref name) = metar.name {
                announcement.push_str(&format!("{}... ", expand_abbreviations(name)));
            }

            announcement.push_str(&format!("Raw METAR... {}... ", metar.raw_ob));

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

            if let Some(wx) = wx_string {
                announcement.push_str(&format!("Weather string... {}... ", wx));
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Weather codes found... ");
                    let code_descriptions: Vec<String> = codes
                        .iter()
                        .map(|c| format!("{} ({})", c.description(), c.code()))
                        .collect();
                    announcement.push_str(&code_descriptions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("No weather codes found...");
                }
            } else {
                announcement.push_str("Weather... none reported... No weather codes found...");
            }

            announcement
        }

        AnnouncementFormat::Aviation => {
            let mut announcement = format!("{} weather... ", spell_out_icao(&metar.icao_id));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature {} degrees... ",
                    spoken_degrees(temp_f)
                ));
            }

            if let Some(wx) = wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    for code in codes {
                        announcement.push_str(&format!("{}... ", code.description()));
                    }
                } else {
                    announcement.push_str("Conditions not recognized... ");
                }
            } else {
                announcement.push_str("No significant weather reported... ");
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

    fn bare_metar() -> MetarData {
        MetarData {
            icao_id: "KSFO".to_string(),
            raw_ob: RAW_OB.to_string(),
            temp: None,
            wx_string: None,
            name: None,
        }
    }

    fn full_metar() -> MetarData {
        MetarData {
            temp: Some(20.0),
            wx_string: Some("-RA BR".to_string()),
            name: Some("San Francisco Intl, CA".to_string()),
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
                "Weather for K S F O... San Francisco International, California... Temperature... 68 degrees fahrenheit... Current conditions... Rain... Mist...",
            ),
            (
                bare_metar(),
                "Weather for K S F O... No significant weather reported...",
            ),
            (
                unrecognized_wx_metar(),
                "Weather for K S F O... Conditions not recognized...",
            ),
        ];

        for (metar, expected) in cases {
            for format in [AnnouncementFormat::Speech, AnnouncementFormat::Brief] {
                assert_eq!(generate_weather_announcement(&metar, &format), expected);
            }
        }
    }

    #[test]
    fn detailed_announcements() {
        let cases = [
            (
                full_metar(),
                format!(
                    "Detailed weather report for K S F O... San Francisco International, California... Raw METAR... {RAW_OB}... Temperature... 68 degrees fahrenheit... 20 degrees celsius... Weather string... -RA BR... Weather codes found... Rain (RA)... Mist (BR)..."
                ),
            ),
            (
                bare_metar(),
                format!(
                    "Detailed weather report for K S F O... Raw METAR... {RAW_OB}... Temperature... not available... Weather... none reported... No weather codes found..."
                ),
            ),
            (
                unrecognized_wx_metar(),
                format!(
                    "Detailed weather report for K S F O... Raw METAR... {RAW_OB}... Temperature... not available... Weather string... XX... No weather codes found..."
                ),
            ),
        ];

        for (metar, expected) in cases {
            assert_eq!(
                generate_weather_announcement(&metar, &AnnouncementFormat::Detailed),
                expected
            );
        }
    }

    #[test]
    fn aviation_announcements() {
        let cases = [
            (
                full_metar(),
                "K S F O weather... Temperature 68 degrees... Rain... Mist... End weather...",
            ),
            (
                bare_metar(),
                "K S F O weather... No significant weather reported... End weather...",
            ),
            (
                unrecognized_wx_metar(),
                "K S F O weather... Conditions not recognized... End weather...",
            ),
        ];

        for (metar, expected) in cases {
            assert_eq!(
                generate_weather_announcement(&metar, &AnnouncementFormat::Aviation),
                expected
            );
        }
    }

    #[test]
    fn unknown_precipitation_is_announced() {
        let metar = MetarData {
            wx_string: Some("UP".to_string()),
            ..bare_metar()
        };

        assert_eq!(
            generate_weather_announcement(&metar, &AnnouncementFormat::Speech),
            "Weather for K S F O... Current conditions... Unknown Precipitation..."
        );
        assert_eq!(
            generate_weather_announcement(&metar, &AnnouncementFormat::Aviation),
            "K S F O weather... Unknown Precipitation... End weather..."
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
            assert_eq!(
                generate_weather_announcement(&metar, &format),
                generate_weather_announcement(&bare_metar(), &format)
            );
        }
    }

    #[test]
    fn temperatures_are_rounded_to_whole_degrees() {
        let metar = MetarData {
            temp: Some(21.5),
            ..bare_metar()
        };

        let announcement = generate_weather_announcement(&metar, &AnnouncementFormat::Detailed);

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
            let announcement = generate_weather_announcement(&metar, &format);
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

        let announcement = generate_weather_announcement(&metar, &AnnouncementFormat::Detailed);

        assert!(
            announcement.contains("31 degrees fahrenheit... 0 degrees celsius"),
            "{announcement}"
        );
    }
}
