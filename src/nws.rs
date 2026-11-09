//! Forecasts, alerts and location identifiers from the US National Weather Service's
//! api.weather.gov.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::str::FromStr;

use super::{Result, WeatherError, geocode, get_json, http_client, validate_icao};

const SERVICE: &str = "api.weather.gov";
const BASE_URL: &str = "https://api.weather.gov";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    pub lat: f64,
    pub lon: f64,
}

impl Coordinates {
    pub fn new(lat: f64, lon: f64) -> Result<Self> {
        if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
            Ok(Self { lat, lon })
        } else {
            Err(WeatherError::InvalidLocation(format!("{lat},{lon}")))
        }
    }
}

/// api.weather.gov redirects requests for points given to more than four decimal places.
fn four_places(degrees: f64) -> String {
    let fixed = format!("{degrees:.4}");
    fixed
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

impl fmt::Display for Coordinates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", four_places(self.lat), four_places(self.lon))
    }
}

/// A place given as an ICAO station, `latitude,longitude`, a US street address (starting with a
/// house number) or any other name, such as "Mountain View, CA".
#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Station(String),
    Coordinates(Coordinates),
    Address(String),
    Name(String),
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Station(id) => write!(f, "{id}"),
            Location::Coordinates(coordinates) => write!(f, "{coordinates}"),
            Location::Address(text) | Location::Name(text) => write!(f, "{text}"),
        }
    }
}

/// A street address starts with a house number, such as "200" in "200 E Santa Clara St".
fn is_street_address(s: &str) -> bool {
    s.split_once(' ').is_some_and(|(number, rest)| {
        !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) && !rest.trim().is_empty()
    })
}

impl FromStr for Location {
    type Err = WeatherError;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(WeatherError::InvalidLocation(s.to_string()));
        }
        if let Some((lat, lon)) = s.split_once(',')
            && let (Ok(lat), Ok(lon)) = (lat.trim().parse(), lon.trim().parse())
        {
            return Coordinates::new(lat, lon).map(Location::Coordinates);
        }
        if validate_icao(s).is_ok() {
            Ok(Location::Station(s.to_uppercase()))
        } else if is_street_address(s) {
            Ok(Location::Address(s.to_string()))
        } else {
            Ok(Location::Name(s.to_string()))
        }
    }
}

/// Where a [`Location`] is, and what the service that found it called it.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub coordinates: Coordinates,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Feature<P> {
    properties: P,
}

#[derive(Debug, Deserialize)]
struct FeatureCollection<P> {
    features: Vec<Feature<P>>,
}

impl<P> FeatureCollection<P> {
    fn into_properties(self) -> Vec<P> {
        self.features.into_iter().map(|f| f.properties).collect()
    }
}

#[derive(Debug, Deserialize)]
struct PointGeometry {
    /// Longitude, then latitude.
    coordinates: (f64, f64),
}

#[derive(Debug, Deserialize)]
struct StationFeature {
    geometry: PointGeometry,
    properties: Station,
}

impl StationFeature {
    fn into_place(self) -> Result<Place> {
        let (lon, lat) = self.geometry.coordinates;
        Ok(Place {
            coordinates: Coordinates::new(lat, lon)?,
            name: Some(self.properties.name),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    pub station_identifier: String,
    pub name: String,
    /// Distance from the point it was looked up for, in metres.
    #[serde(default, deserialize_with = "quantity_value")]
    pub distance: Option<f64>,
}

#[derive(Deserialize)]
struct Quantity {
    value: Option<f64>,
}

fn quantity_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<f64>, D::Error> {
    Ok(Option::<Quantity>::deserialize(deserializer)?.and_then(|q| q.value))
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelativeLocation {
    pub city: String,
    pub state: String,
}

/// The NWS grid square and zones that cover a point.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    #[serde(rename = "cwa")]
    pub office: String,
    pub grid_id: String,
    pub grid_x: u32,
    pub grid_y: u32,
    forecast: Option<String>,
    observation_stations: String,
    forecast_zone: Option<String>,
    county: Option<String>,
    relative_location: Option<Feature<RelativeLocation>>,
    pub time_zone: Option<String>,
}

/// The identifier at the end of an api.weather.gov URL, such as `CAZ513` in
/// `https://api.weather.gov/zones/forecast/CAZ513`.
fn url_id(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

impl Point {
    pub fn forecast_zone(&self) -> Option<&str> {
        self.forecast_zone.as_deref().map(url_id)
    }

    pub fn county_zone(&self) -> Option<&str> {
        self.county.as_deref().map(url_id)
    }

    pub fn nearest_city(&self) -> Option<&RelativeLocation> {
        self.relative_location.as_ref().map(|f| &f.properties)
    }

    /// Such as `MTR 98,83`.
    pub fn grid(&self) -> String {
        format!("{} {},{}", self.grid_id, self.grid_x, self.grid_y)
    }
}

#[derive(Debug, Deserialize)]
struct Forecast {
    periods: Vec<ForecastPeriod>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForecastPeriod {
    /// Such as "Tonight" or "Veterans Day".
    pub name: String,
    pub detailed_forecast: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Alert {
    /// Such as "Wind Advisory".
    pub event: String,
    pub headline: Option<String>,
}

pub struct NwsClient {
    http: reqwest::blocking::Client,
}

impl NwsClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: http_client()?,
        })
    }

    fn get<T: DeserializeOwned>(&self, url: &str, query: &[(&str, &str)]) -> Result<T> {
        get_json(&self.http, SERVICE, url, query)
    }

    /// Finds stations with api.weather.gov, addresses with the US Census geocoder and other names
    /// with Nominatim.
    pub fn locate(&self, location: &Location) -> Result<Place> {
        match location {
            Location::Coordinates(coordinates) => Ok(Place {
                coordinates: *coordinates,
                name: None,
            }),
            Location::Station(id) => self
                .get::<StationFeature>(&format!("{BASE_URL}/stations/{id}"), &[])
                .map_err(|e| match e {
                    WeatherError::HttpStatus { status, .. }
                        if status == reqwest::StatusCode::NOT_FOUND =>
                    {
                        WeatherError::UnknownStation(id.clone())
                    }
                    e => e,
                })?
                .into_place(),
            Location::Address(address) => geocode::locate_address(&self.http, address),
            Location::Name(name) => geocode::locate_name(&self.http, name),
        }
    }

    pub fn point(&self, coordinates: Coordinates) -> Result<Point> {
        let point: Feature<Point> = self.get(&format!("{BASE_URL}/points/{coordinates}"), &[])?;
        Ok(point.properties)
    }

    /// The observation stations nearest the point, nearest first.
    pub fn stations(&self, point: &Point, limit: usize) -> Result<Vec<Station>> {
        let stations: FeatureCollection<Station> = self.get(
            &point.observation_stations,
            &[("limit", &limit.to_string())],
        )?;
        Ok(stations.into_properties())
    }

    pub fn forecast(&self, point: &Point) -> Result<Vec<ForecastPeriod>> {
        let url = point
            .forecast
            .as_deref()
            .ok_or_else(|| WeatherError::NoForecast(point.grid()))?;
        let forecast: Feature<Forecast> = self.get(url, &[])?;
        if forecast.properties.periods.is_empty() {
            return Err(WeatherError::NoForecast(point.grid()));
        }
        Ok(forecast.properties.periods)
    }

    pub fn alerts(&self, coordinates: Coordinates) -> Result<Vec<Alert>> {
        let alerts: FeatureCollection<Alert> = self.get(
            &format!("{BASE_URL}/alerts/active"),
            &[
                ("point", &coordinates.to_string()),
                ("status", "actual"),
                ("message_type", "alert,update"),
            ],
        )?;
        Ok(alerts.into_properties())
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse_json;
    use super::*;

    fn coordinates(lat: f64, lon: f64) -> Coordinates {
        Coordinates::new(lat, lon).unwrap()
    }

    #[test]
    fn parses_locations() {
        let cases = [
            ("KSJC", Location::Station("KSJC".to_string())),
            (" ksjc ", Location::Station("KSJC".to_string())),
            (
                "37.3592,-121.9242",
                Location::Coordinates(coordinates(37.3592, -121.9242)),
            ),
            ("37, -122", Location::Coordinates(coordinates(37.0, -122.0))),
            (
                "200 E Santa Clara St, San Jose, CA",
                Location::Address("200 E Santa Clara St, San Jose, CA".to_string()),
            ),
            (
                "Mountain View, CA",
                Location::Name("Mountain View, CA".to_string()),
            ),
            ("94043", Location::Name("94043".to_string())),
            ("Route 66", Location::Name("Route 66".to_string())),
        ];

        for (input, expected) in cases {
            assert_eq!(input.parse::<Location>().unwrap(), expected, "{input:?}");
        }
    }

    #[test]
    fn rejects_empty_locations_and_impossible_coordinates() {
        for input in ["", "  ", "91,0", "0,181", "NaN,0", "inf,0"] {
            assert!(
                matches!(
                    input.parse::<Location>(),
                    Err(WeatherError::InvalidLocation(_))
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn coordinates_are_given_to_at_most_four_places() {
        let cases = [
            ((37.359_17, -121.924_17), "37.3592,-121.9242"),
            ((37.3, -122.0), "37.3,-122"),
            ((0.0, 100.0), "0,100"),
        ];

        for ((lat, lon), expected) in cases {
            assert_eq!(coordinates(lat, lon).to_string(), expected);
        }
    }

    #[test]
    fn parses_a_station() {
        let station: StationFeature = parse_json(include_str!("testdata/station.json")).unwrap();

        assert_eq!(
            station.into_place().unwrap(),
            Place {
                coordinates: coordinates(37.35917, -121.92417),
                name: Some("San Jose, San Jose International Airport".to_string()),
            }
        );
    }

    #[test]
    fn parses_a_point() {
        let point = parse_json::<Feature<Point>>(include_str!("testdata/points.json"))
            .unwrap()
            .properties;

        assert_eq!(point.office, "MTR");
        assert_eq!(point.grid(), "MTR 98,83");
        assert_eq!(point.forecast_zone(), Some("CAZ513"));
        assert_eq!(point.county_zone(), Some("CAC085"));
        assert_eq!(point.time_zone.as_deref(), Some("America/Los_Angeles"));
        let city = point.nearest_city().unwrap();
        assert_eq!(
            (city.city.as_str(), city.state.as_str()),
            ("San Jose", "CA")
        );
        assert_eq!(
            point.forecast.as_deref(),
            Some("https://api.weather.gov/gridpoints/MTR/98,83/forecast")
        );
        assert_eq!(
            point.observation_stations,
            "https://api.weather.gov/gridpoints/MTR/98,83/stations"
        );
    }

    #[test]
    fn parses_nearby_stations() {
        let stations =
            parse_json::<FeatureCollection<Station>>(include_str!("testdata/stations.json"))
                .unwrap()
                .into_properties();

        let ids: Vec<&str> = stations
            .iter()
            .map(|s| s.station_identifier.as_str())
            .collect();
        assert_eq!(ids, ["KSJC", "KRHV", "KNUQ", "LSGC1", "AW020"]);
        assert_eq!(stations[0].name, "San Jose, San Jose International Airport");
        assert_eq!(stations[0].distance.map(f64::round), Some(1093.0));
    }

    #[test]
    fn parses_a_forecast() {
        let periods = parse_json::<Feature<Forecast>>(include_str!("testdata/forecast.json"))
            .unwrap()
            .properties
            .periods;

        assert_eq!(periods.len(), 14);
        assert_eq!(periods[1].name, "Tonight");
        assert_eq!(
            periods[1].detailed_forecast,
            "Patchy fog after 1am. Partly cloudy, with a low around 51. East southeast wind 0 to 5 mph."
        );
    }

    #[test]
    fn parses_alerts() {
        let alerts = parse_json::<FeatureCollection<Alert>>(include_str!("testdata/alerts.json"))
            .unwrap()
            .into_properties();

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].event, "Wind Advisory");
        assert_eq!(
            alerts[0].headline.as_deref(),
            Some(
                "Wind Advisory issued November 8 at 3:02PM AKST until November 10 at 6:00PM AKST by NWS Fairbanks AK"
            )
        );
    }

    #[test]
    fn parses_no_alerts() {
        let alerts: FeatureCollection<Alert> = parse_json(
            r#"{"type": "FeatureCollection", "features": [], "title": "Current watches, warnings, and advisories for 37.3592 N, 121.9242 W"}"#,
        )
        .unwrap();

        assert!(alerts.into_properties().is_empty());
    }
}
