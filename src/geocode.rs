//! US street addresses located with the US Census geocoder, which needs no API key.

use serde::Deserialize;

use super::nws::{Coordinates, Place};
use super::{Result, WeatherError, get_json};

const SERVICE: &str = "geocoding.geo.census.gov";
const URL: &str = "https://geocoding.geo.census.gov/geocoder/locations/onelineaddress";

#[derive(Debug, Deserialize)]
struct Response {
    result: Matches,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Matches {
    address_matches: Vec<AddressMatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddressMatch {
    coordinates: Xy,
    matched_address: String,
}

#[derive(Debug, Deserialize)]
struct Xy {
    x: f64,
    y: f64,
}

pub(super) fn locate(client: &reqwest::blocking::Client, address: &str) -> Result<Place> {
    let response = get_json(
        client,
        SERVICE,
        URL,
        &[
            ("benchmark", "Public_AR_Current"),
            ("format", "json"),
            ("address", address),
        ],
    )?;
    best_match(address, response)
}

fn best_match(address: &str, response: Response) -> Result<Place> {
    let best = response
        .result
        .address_matches
        .into_iter()
        .next()
        .ok_or_else(|| WeatherError::AddressNotFound(address.to_string()))?;
    Ok(Place {
        coordinates: Coordinates::new(best.coordinates.y, best.coordinates.x)?,
        name: Some(best.matched_address),
    })
}

#[cfg(test)]
mod tests {
    use super::super::parse_json;
    use super::*;

    #[test]
    fn locates_a_matched_address() {
        let response = parse_json(include_str!("testdata/geocode.json")).unwrap();

        let place = best_match("200 E Santa Clara St, San Jose, CA 95113", response).unwrap();

        assert_eq!(
            place.name.as_deref(),
            Some("200 E SANTA CLARA ST, SAN JOSE, CA, 95113")
        );
        assert_eq!(place.coordinates.to_string(), "37.3382,-121.8862");
    }

    #[test]
    fn reports_an_unmatched_address() {
        let response = parse_json(r#"{"result": {"input": {}, "addressMatches": []}}"#).unwrap();

        assert!(matches!(
            best_match("nowhere", response),
            Err(WeatherError::AddressNotFound(address)) if address == "nowhere"
        ));
    }
}
