use serde::Deserialize;
use std::fmt;

mod abbreviations;
pub mod tts;
mod weather_groups;

pub use abbreviations::{expand_abbreviations, expand_station_name};
pub use weather_groups::{WeatherGroup, parse_weather_groups};

#[derive(Debug)]
pub enum WeatherError {
    InvalidIcao(String),
    HttpClient(String),
    Request(String),
    EmptyResponse(String),
    InvalidJson(String),
    NoData(String),
}

impl fmt::Display for WeatherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeatherError::InvalidIcao(icao) => write!(
                f,
                "Invalid ICAO code '{}': expected 4 letters or digits (e.g., KRHV)",
                icao
            ),
            WeatherError::HttpClient(msg) => write!(f, "HTTP client error: {}", msg),
            WeatherError::Request(msg) => write!(f, "Request failed: {}", msg),
            WeatherError::EmptyResponse(icao) => write!(
                f,
                "Empty response from API. ICAO code '{}' may not be valid or may not have current weather data. Try adding 'K' prefix for US airports (e.g., KRHV)",
                icao
            ),
            WeatherError::InvalidJson(msg) => write!(f, "Failed to parse JSON response: {}", msg),
            WeatherError::NoData(icao) => write!(
                f,
                "No weather data found for ICAO: {}. This airport may not report METAR data or may not be a valid ICAO identifier.\nCommon reasons:\n- Small airports may not have weather reporting\n- Try the full ICAO code (US airports: add 'K' prefix, e.g., KRHV)\n- Verify the airport code at https://aviationweather.gov",
                icao
            ),
        }
    }
}

impl std::error::Error for WeatherError {}

pub type Result<T> = std::result::Result<T, WeatherError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WmoCode {
    // Precipitation
    Rain,                 // RA
    Snow,                 // SN
    Drizzle,              // DZ
    SnowGrains,           // SG
    IceCrystals,          // IC
    IcePellets,           // PL
    Hail,                 // GR
    SmallHail,            // GS
    UnknownPrecipitation, // UP

    // Obscuration
    Fog,         // FG
    Mist,        // BR
    Haze,        // HZ
    Smoke,       // FU
    VolcanicAsh, // VA
    Dust,        // DU
    Sand,        // SA
    Spray,       // PY

    // Other phenomena
    Thunderstorm, // TS
    Squall,       // SQ
    FunnelCloud,  // FC
    Sandstorm,    // SS
    Duststorm,    // DS
    DustDevils,   // PO
}

impl WmoCode {
    pub fn code(&self) -> &str {
        match self {
            WmoCode::Rain => "RA",
            WmoCode::Snow => "SN",
            WmoCode::Drizzle => "DZ",
            WmoCode::SnowGrains => "SG",
            WmoCode::IceCrystals => "IC",
            WmoCode::IcePellets => "PL",
            WmoCode::Hail => "GR",
            WmoCode::SmallHail => "GS",
            WmoCode::UnknownPrecipitation => "UP",
            WmoCode::Fog => "FG",
            WmoCode::Mist => "BR",
            WmoCode::Haze => "HZ",
            WmoCode::Smoke => "FU",
            WmoCode::VolcanicAsh => "VA",
            WmoCode::Dust => "DU",
            WmoCode::Sand => "SA",
            WmoCode::Spray => "PY",
            WmoCode::Thunderstorm => "TS",
            WmoCode::Squall => "SQ",
            WmoCode::FunnelCloud => "FC",
            WmoCode::Sandstorm => "SS",
            WmoCode::Duststorm => "DS",
            WmoCode::DustDevils => "PO",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            WmoCode::Rain => "Rain",
            WmoCode::Snow => "Snow",
            WmoCode::Drizzle => "Drizzle",
            WmoCode::SnowGrains => "Snow Grains",
            WmoCode::IceCrystals => "Ice Crystals",
            WmoCode::IcePellets => "Ice Pellets",
            WmoCode::Hail => "Hail",
            WmoCode::SmallHail => "Small Hail/Snow Pellets",
            WmoCode::UnknownPrecipitation => "Unknown Precipitation",
            WmoCode::Fog => "Fog",
            WmoCode::Mist => "Mist",
            WmoCode::Haze => "Haze",
            WmoCode::Smoke => "Smoke",
            WmoCode::VolcanicAsh => "Volcanic Ash",
            WmoCode::Dust => "Dust",
            WmoCode::Sand => "Sand",
            WmoCode::Spray => "Spray",
            WmoCode::Thunderstorm => "Thunderstorm",
            WmoCode::Squall => "Squall",
            WmoCode::FunnelCloud => "Funnel Cloud/Tornado/Waterspout",
            WmoCode::Sandstorm => "Sandstorm",
            WmoCode::Duststorm => "Duststorm",
            WmoCode::DustDevils => "Dust/Sand Whirls",
        }
    }

    pub fn from_code(code: &str) -> Option<WmoCode> {
        Self::all_codes().into_iter().find(|c| c.code() == code)
    }

    fn all_codes() -> Vec<WmoCode> {
        vec![
            WmoCode::Rain,
            WmoCode::Snow,
            WmoCode::Drizzle,
            WmoCode::SnowGrains,
            WmoCode::IceCrystals,
            WmoCode::IcePellets,
            WmoCode::Hail,
            WmoCode::SmallHail,
            WmoCode::UnknownPrecipitation,
            WmoCode::Fog,
            WmoCode::Mist,
            WmoCode::Haze,
            WmoCode::Smoke,
            WmoCode::VolcanicAsh,
            WmoCode::Dust,
            WmoCode::Sand,
            WmoCode::Spray,
            WmoCode::Thunderstorm,
            WmoCode::Squall,
            WmoCode::FunnelCloud,
            WmoCode::Sandstorm,
            WmoCode::Duststorm,
            WmoCode::DustDevils,
        ]
    }
}

impl fmt::Display for WmoCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.code(), self.description())
    }
}

#[derive(Debug, Deserialize)]
pub struct MetarData {
    #[serde(rename = "icaoId")]
    #[allow(dead_code)]
    pub icao_id: String,
    #[serde(rename = "rawOb")]
    pub raw_ob: String,
    /// Time of the observation, in seconds since the Unix epoch.
    #[serde(rename = "obsTime")]
    pub obs_time: Option<i64>,
    pub temp: Option<f64>,
    #[serde(rename = "wxString")]
    pub wx_string: Option<String>,
    pub name: Option<String>,
}

pub fn celsius_to_fahrenheit(c: f64) -> f64 {
    (c * 9.0 / 5.0) + 32.0
}

/// The API also accepts station lists and state selectors, and would then report another airport.
fn validate_icao(icao: &str) -> Result<()> {
    if icao.len() == 4 && icao.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(WeatherError::InvalidIcao(icao.to_string()))
    }
}

pub fn fetch_weather_data(icao: &str) -> Result<MetarData> {
    validate_icao(icao)?;

    let url = format!(
        "https://aviationweather.gov/api/data/metar?ids={}&format=json",
        icao.to_uppercase()
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("aviation-weather-cli/0.1.0")
        .build()
        .map_err(|e| WeatherError::HttpClient(e.to_string()))?;

    let response = client
        .get(&url)
        .send()
        .map_err(|e| WeatherError::Request(e.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|e| WeatherError::Request(e.to_string()))?;

    parse_metar_response(icao, status, &body)
}

fn parse_metar_response(icao: &str, status: reqwest::StatusCode, body: &str) -> Result<MetarData> {
    if !status.is_success() {
        return Err(WeatherError::Request(format!(
            "aviationweather.gov returned HTTP status {status}"
        )));
    }

    if body.is_empty() {
        return Err(WeatherError::EmptyResponse(icao.to_string()));
    }

    let response: Vec<MetarData> = serde_json::from_str(body)
        .map_err(|e| WeatherError::InvalidJson(format!("{}: {}", e, body)))?;

    response
        .into_iter()
        .next()
        .ok_or_else(|| WeatherError::NoData(icao.to_uppercase()))
}

pub fn display_weather(metar: &MetarData) {
    println!("Raw METAR: {}", metar.raw_ob);
    if let Some(ref name) = metar.name {
        println!("Station: {}", name);
    }
    println!();

    if let Some(temp_c) = metar.temp {
        let temp_f = celsius_to_fahrenheit(temp_c);
        println!("Temperature: {:.1}°F ({:.1}°C)", temp_f, temp_c);
    } else {
        println!("Temperature: Not available");
    }

    if let Some(ref wx) = metar.wx_string {
        println!("Weather String: {}", wx);
    } else {
        println!("Weather: None reported");
    }

    let groups = metar
        .wx_string
        .as_deref()
        .map(parse_weather_groups)
        .unwrap_or_default();
    if groups.is_empty() {
        println!("WMO Codes Found: None");
    } else {
        println!("WMO Codes Found:");
        for group in groups {
            let description = group.description.as_deref().unwrap_or("Not understood");
            println!("  - {} ({})", group.raw, description);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_full_metar_ignoring_unknown_fields() {
        let metar: MetarData = serde_json::from_str(
            r#"{
                "icaoId": "KSFO",
                "rawOb": "METAR KSFO 061256Z -RA BR",
                "obsTime": 1793983980,
                "temp": 12.5,
                "wxString": "-RA BR",
                "name": "San Francisco Intl, CA, US",
                "elev": 3
            }"#,
        )
        .unwrap();

        assert_eq!(metar.icao_id, "KSFO");
        assert_eq!(metar.raw_ob, "METAR KSFO 061256Z -RA BR");
        assert_eq!(metar.obs_time, Some(1793983980));
        assert_eq!(metar.temp, Some(12.5));
        assert_eq!(metar.wx_string.as_deref(), Some("-RA BR"));
        assert_eq!(metar.name.as_deref(), Some("San Francisco Intl, CA, US"));
    }

    #[test]
    fn deserializes_missing_and_null_optional_fields_as_none() {
        let documents = [
            r#"{"icaoId": "KSFO", "rawOb": "RAW"}"#,
            r#"{"icaoId": "KSFO", "rawOb": "RAW", "temp": null, "wxString": null, "name": null}"#,
        ];

        for document in documents {
            let metar: MetarData = serde_json::from_str(document).unwrap();
            assert_eq!(metar.temp, None, "document: {document}");
            assert_eq!(metar.wx_string, None, "document: {document}");
            assert_eq!(metar.name, None, "document: {document}");
        }
    }

    #[test]
    fn rejects_metar_missing_required_fields() {
        for document in [r#"{"icaoId": "KSFO"}"#, r#"{"rawOb": "RAW"}"#] {
            assert!(
                serde_json::from_str::<MetarData>(document).is_err(),
                "document: {document}"
            );
        }
    }

    #[test]
    fn accepts_icao_codes() {
        for icao in ["KSJC", "ksjc", "K0Q9", "EGLL"] {
            assert!(validate_icao(icao).is_ok(), "icao: {icao:?}");
        }
    }

    #[test]
    fn rejects_input_the_api_would_treat_as_something_else() {
        for icao in [
            "",
            "K",
            "SJC",
            "KSJCX",
            "KSJC,KSFO",
            "@ca",
            " KSJ",
            "K&=1",
            "KSJé",
        ] {
            assert!(
                matches!(validate_icao(icao), Err(WeatherError::InvalidIcao(_))),
                "icao: {icao:?}"
            );
        }
    }

    #[test]
    fn parses_successful_responses() {
        let metar = parse_metar_response(
            "KSJC",
            reqwest::StatusCode::OK,
            r#"[{"icaoId": "KSJC", "rawOb": "RAW"}]"#,
        )
        .unwrap();

        assert_eq!(metar.icao_id, "KSJC");
    }

    #[test]
    fn classifies_failed_responses() {
        use reqwest::StatusCode;

        let error = |status, body| parse_metar_response("ksjc", status, body).unwrap_err();

        assert!(matches!(
            error(StatusCode::BAD_REQUEST, r#"{"status":"error"}"#),
            WeatherError::Request(_)
        ));
        assert!(matches!(
            error(StatusCode::INTERNAL_SERVER_ERROR, "<html>"),
            WeatherError::Request(_)
        ));
        assert!(matches!(
            error(StatusCode::NO_CONTENT, ""),
            WeatherError::EmptyResponse(_)
        ));
        assert!(matches!(
            error(StatusCode::OK, "[]"),
            WeatherError::NoData(icao) if icao == "KSJC"
        ));
        assert!(matches!(
            error(StatusCode::OK, "<html>"),
            WeatherError::InvalidJson(_)
        ));
    }
}
