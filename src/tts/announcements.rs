use crate::{MetarData, celsius_to_fahrenheit, expand_abbreviations, parse_wmo_codes};

#[derive(Debug, Clone)]
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

pub fn generate_weather_announcement(metar: &MetarData, format: &AnnouncementFormat) -> String {
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
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    announcement.push_str("Current conditions... ");
                    let conditions: Vec<String> =
                        codes.iter().map(|c| c.description().to_string()).collect();
                    announcement.push_str(&conditions.join("... "));
                    announcement.push_str("...");
                } else {
                    announcement.push_str("Clear conditions...");
                }
            } else {
                announcement.push_str("Clear conditions...");
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
                    temp_f.round() as i32,
                    temp_c.round() as i32
                ));
            } else {
                announcement.push_str("Temperature... not available... ");
            }

            if let Some(ref wx) = metar.wx_string {
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
                announcement
                    .push_str("Weather... clear or not reported... No weather codes found...");
            }

            announcement
        }

        AnnouncementFormat::Aviation => {
            let mut announcement = format!("{} weather... ", spell_out_icao(&metar.icao_id));

            if let Some(temp_c) = metar.temp {
                let temp_f = celsius_to_fahrenheit(temp_c);
                announcement.push_str(&format!(
                    "Temperature {} degrees... ",
                    temp_f.round() as i32
                ));
            }

            if let Some(ref wx) = metar.wx_string {
                let codes = parse_wmo_codes(wx);
                if !codes.is_empty() {
                    for code in codes {
                        announcement.push_str(&format!("{}... ", code.description()));
                    }
                } else {
                    announcement.push_str("Clear... ");
                }
            } else {
                announcement.push_str("Clear... ");
            }

            announcement.push_str("End weather...");
            announcement
        }
    }
}
