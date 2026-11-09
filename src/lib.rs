use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::time::Duration;

mod abbreviations;
mod geocode;
pub mod nws;
pub mod report;
pub mod tts;
mod weather_groups;

pub use abbreviations::{expand_abbreviations, expand_station_name};
pub use weather_groups::{WeatherGroup, parse_weather_groups};

#[derive(Debug)]
pub enum WeatherError {
    InvalidIcao(String),
    HttpClient(String),
    Request(String),
    HttpStatus {
        service: &'static str,
        status: reqwest::StatusCode,
        detail: Option<String>,
    },
    EmptyResponse(String),
    InvalidJson(String),
    NoData(String),
    InvalidLocation(String),
    AddressNotFound(String),
    PlaceNotFound(String),
    UnknownStation(String),
    NoForecast(String),
    Usage(String),
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
            WeatherError::HttpStatus {
                service,
                status,
                detail,
            } => {
                write!(f, "{service} returned HTTP status {status}")?;
                match detail {
                    Some(detail) => write!(f, ": {detail}"),
                    None => Ok(()),
                }
            }
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
            WeatherError::InvalidLocation(location) => write!(
                f,
                "Invalid location '{location}': expected a station, latitude,longitude, a US street address or a place name"
            ),
            WeatherError::AddressNotFound(address) => {
                write!(f, "The Census geocoder found no match for '{address}'")
            }
            WeatherError::PlaceNotFound(name) => {
                write!(f, "OpenStreetMap found no place in the US called '{name}'")
            }
            WeatherError::UnknownStation(id) => write!(
                f,
                "{id} is not a National Weather Service station; they are only in the US"
            ),
            WeatherError::NoForecast(point) => {
                write!(f, "api.weather.gov has no forecast for {point}")
            }
            WeatherError::Usage(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for WeatherError {}

impl WeatherError {
    /// Whether the same request might succeed if tried again.
    fn is_transient(&self) -> bool {
        match self {
            WeatherError::Request(_) => true,
            WeatherError::HttpStatus { status, .. } => {
                status.is_server_error() || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            _ => false,
        }
    }
}

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

const USER_AGENT: &str = concat!(
    "weather-rs/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/charlieh0tel/weather-rs)"
);
const RETRIES: u32 = 2;
const RETRY_DELAY: Duration = Duration::from_secs(2);

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

    let url = url_with_query(
        "https://aviationweather.gov/api/data/metar",
        &[("ids", &icao.to_uppercase()), ("format", "json")],
    )?;

    let client = http_client()?;
    with_retries(RETRY_DELAY, || {
        let (status, body) = get(&client, &url)?;
        parse_metar_response(icao, status, &body)
    })
}

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| WeatherError::HttpClient(e.to_string()))
}

fn url_with_query(url: &str, query: &[(&str, &str)]) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(url).map_err(|e| WeatherError::HttpClient(e.to_string()))?;
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    Ok(url)
}

fn get(
    client: &reqwest::blocking::Client,
    url: &reqwest::Url,
) -> Result<(reqwest::StatusCode, String)> {
    let response = client
        .get(url.clone())
        .send()
        .map_err(|e| WeatherError::Request(e.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|e| WeatherError::Request(e.to_string()))?;
    Ok((status, body))
}

fn get_json<T: DeserializeOwned>(
    client: &reqwest::blocking::Client,
    service: &'static str,
    url: &str,
    query: &[(&str, &str)],
) -> Result<T> {
    let url = url_with_query(url, query)?;
    with_retries(RETRY_DELAY, || {
        let (status, body) = get(client, &url)?;
        check_status(service, status, &body)?;
        parse_json(&body)
    })
}

/// An error body: an RFC 7807 problem from api.weather.gov, or the Census geocoder's error list.
#[derive(Deserialize)]
struct Problem {
    detail: Option<String>,
    #[serde(default)]
    errors: Vec<String>,
}

fn check_status(service: &'static str, status: reqwest::StatusCode, body: &str) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    let detail = serde_json::from_str::<Problem>(body)
        .ok()
        .and_then(|problem| {
            problem
                .detail
                .or_else(|| (!problem.errors.is_empty()).then(|| problem.errors.join("; ")))
        })
        .filter(|detail| Some(detail.as_str()) != status.canonical_reason());
    Err(WeatherError::HttpStatus {
        service,
        status,
        detail,
    })
}

fn parse_json<T: DeserializeOwned>(body: &str) -> Result<T> {
    serde_json::from_str(body).map_err(|e| WeatherError::InvalidJson(format!("{}: {}", e, body)))
}

fn with_retries<T>(delay: Duration, mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
    for _ in 0..RETRIES {
        match attempt() {
            Err(e) if e.is_transient() => {
                eprintln!("{e}; retrying in {} seconds", delay.as_secs());
                std::thread::sleep(delay);
            }
            result => return result,
        }
    }
    attempt()
}

fn parse_metar_response(icao: &str, status: reqwest::StatusCode, body: &str) -> Result<MetarData> {
    check_status("aviationweather.gov", status, body)?;

    if body.is_empty() {
        return Err(WeatherError::EmptyResponse(icao.to_string()));
    }

    let response: Vec<MetarData> = parse_json(body)?;

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
            WeatherError::HttpStatus { .. }
        ));
        assert!(matches!(
            error(StatusCode::INTERNAL_SERVER_ERROR, "<html>"),
            WeatherError::HttpStatus { .. }
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

    #[test]
    fn encodes_query_parameters() {
        let url = |query| url_with_query("https://example.com/a/1,2", query).unwrap();

        assert_eq!(url(&[]).as_str(), "https://example.com/a/1,2");
        assert_eq!(
            url(&[("point", "37.3,-122"), ("address", "200 E St, San Jose")]).as_str(),
            "https://example.com/a/1,2?point=37.3%2C-122&address=200+E+St%2C+San+Jose"
        );
    }

    #[test]
    fn reports_the_detail_of_failed_responses() {
        let detail = |body| match check_status("test", reqwest::StatusCode::NOT_FOUND, body) {
            Err(WeatherError::HttpStatus { detail, .. }) => detail,
            other => panic!("{other:?}"),
        };

        assert_eq!(
            detail(r#"{"title": "Not Found", "detail": "Unable to provide data"}"#).as_deref(),
            Some("Unable to provide data")
        );
        assert_eq!(
            detail(r#"{"errors": ["Address cannot be empty", "Bad benchmark"]}"#).as_deref(),
            Some("Address cannot be empty; Bad benchmark")
        );
        assert_eq!(detail(r#"{"detail": "Not Found"}"#), None);
        assert_eq!(detail("<html>"), None);
        assert_eq!(detail("{}"), None);
    }

    fn count_attempts(mut outcome: impl FnMut(u32) -> Result<()>) -> (u32, Result<()>) {
        let mut attempts = 0;
        let result = with_retries(Duration::ZERO, || {
            attempts += 1;
            outcome(attempts)
        });
        (attempts, result)
    }

    fn http_status(status: reqwest::StatusCode) -> WeatherError {
        WeatherError::HttpStatus {
            service: "test",
            status,
            detail: None,
        }
    }

    fn gateway_timeout() -> WeatherError {
        http_status(reqwest::StatusCode::GATEWAY_TIMEOUT)
    }

    #[test]
    fn retries_until_a_transient_failure_clears() {
        let (attempts, result) = count_attempts(|attempt| match attempt {
            1 => Err(gateway_timeout()),
            2 => Err(WeatherError::Request("connection reset".to_string())),
            _ => Ok(()),
        });

        assert_eq!(attempts, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn gives_up_after_two_retries() {
        let (attempts, result) = count_attempts(|_| Err(gateway_timeout()));

        assert_eq!(attempts, 3);
        assert!(matches!(result, Err(WeatherError::HttpStatus { .. })));
    }

    #[test]
    fn stops_at_the_first_success() {
        assert_eq!(count_attempts(|_| Ok(())).0, 1);
    }

    #[test]
    fn does_not_retry_failures_that_would_repeat() {
        let failures: [fn() -> WeatherError; 4] = [
            || http_status(reqwest::StatusCode::BAD_REQUEST),
            || WeatherError::NoData("KSJC".to_string()),
            || WeatherError::EmptyResponse("KSJC".to_string()),
            || WeatherError::InvalidJson("<html>".to_string()),
        ];

        for failure in failures {
            assert_eq!(count_attempts(|_| Err(failure())).0, 1);
        }
    }

    #[test]
    fn retries_when_rate_limited() {
        let rate_limited = http_status(reqwest::StatusCode::TOO_MANY_REQUESTS);

        assert!(rate_limited.is_transient());
    }
}
