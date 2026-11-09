//! Places located by name: US street addresses with the US Census geocoder, and anything else
//! with OpenStreetMap's Nominatim.  Neither needs an API key.

use serde::Deserialize;

use super::nws::{Coordinates, Place};
use super::{Result, WeatherError, get_json};

const CENSUS: &str = "geocoding.geo.census.gov";
const CENSUS_URL: &str = "https://geocoding.geo.census.gov/geocoder/locations/onelineaddress";
const NOMINATIM: &str = "nominatim.openstreetmap.org";
const NOMINATIM_URL: &str = "https://nominatim.openstreetmap.org/search";

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

pub(super) fn locate_address(client: &reqwest::blocking::Client, address: &str) -> Result<Place> {
    let response = get_json(
        client,
        CENSUS,
        CENSUS_URL,
        &[
            ("benchmark", "Public_AR_Current"),
            ("format", "json"),
            ("address", address),
        ],
    )?;
    best_match(address, response)
}

#[derive(Debug, Deserialize)]
struct NominatimPlace {
    lat: String,
    lon: String,
    display_name: String,
}

/// Locates a place by name, such as "Mountain View, CA", in the US.
pub(super) fn locate_name(client: &reqwest::blocking::Client, name: &str) -> Result<Place> {
    let places = get_json(
        client,
        NOMINATIM,
        NOMINATIM_URL,
        &[
            ("q", name),
            ("format", "jsonv2"),
            ("limit", "1"),
            ("countrycodes", "us"),
        ],
    )?;
    first_place(name, places)
}

fn first_place(name: &str, places: Vec<NominatimPlace>) -> Result<Place> {
    let place = places
        .into_iter()
        .next()
        .ok_or_else(|| WeatherError::PlaceNotFound(name.to_string()))?;
    let degrees = |value: &str| {
        value.parse().map_err(|_| {
            WeatherError::InvalidJson(format!("{NOMINATIM} gave '{value}' as a coordinate"))
        })
    };
    Ok(Place {
        coordinates: Coordinates::new(degrees(&place.lat)?, degrees(&place.lon)?)?,
        name: Some(place.display_name),
    })
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
    fn locates_a_place_by_name() {
        let places = parse_json(include_str!("testdata/nominatim.json")).unwrap();

        let place = first_place("Mountain View, CA", places).unwrap();

        assert_eq!(
            place.name.as_deref(),
            Some("Mountain View, Santa Clara County, California, United States")
        );
        assert_eq!(place.coordinates.to_string(), "37.3894,-122.0832");
        assert!(matches!(
            first_place("Nowhere", Vec::new()),
            Err(WeatherError::PlaceNotFound(name)) if name == "Nowhere"
        ));
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
