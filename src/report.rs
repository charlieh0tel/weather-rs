//! The reports wx announces, for a place.

use crate::{
    MetarData, Result, WeatherError, fetch_weather_data,
    nws::{Location, NwsClient},
    tts::{alerts_announcement, forecast_announcement},
};

pub const NOTHING_TO_ANNOUNCE: &str = "No weather alerts in effect; nothing to announce.";

/// The METAR station for a place: the station given, or else the nearest National Weather
/// Service observation station with an ICAO identifier.
pub fn metar_station(place: &Location) -> Result<String> {
    if let Location::Station(id) = place {
        return Ok(id.clone());
    }
    let nws = NwsClient::new()?;
    let point = nws.point(nws.locate(place)?.coordinates)?;
    let station = nws
        .stations(&point, 10)?
        .into_iter()
        .find(|station| crate::validate_icao(&station.station_identifier).is_ok())
        .ok_or_else(|| WeatherError::NoMetarStation(place.to_string()))?;
    let distance = station
        .distance
        .map(|metres| format!(", {:.1} km away", metres / 1000.0))
        .unwrap_or_default();
    eprintln!(
        "Using {} ({}{distance})",
        station.station_identifier, station.name
    );
    Ok(station.station_identifier)
}

/// The latest METAR for a place.
pub fn metar(place: &Location) -> Result<MetarData> {
    fetch_weather_data(&metar_station(place)?)
}

/// The National Weather Service forecast for a place, as text to speak.  With `with_alerts`, any
/// active alerts come first; when there are none, nothing is said about them.
pub fn nws_forecast(place: &Location, periods: usize, with_alerts: bool) -> Result<String> {
    let nws = NwsClient::new()?;
    let coordinates = nws.locate(place)?.coordinates;
    let alerts = if with_alerts {
        nws.alerts(coordinates)?
    } else {
        Vec::new()
    };
    let forecast = nws.forecast(&nws.point(coordinates)?)?;
    let forecast = forecast_announcement(&forecast[..periods.min(forecast.len())]);
    if alerts.is_empty() {
        return Ok(forecast);
    }
    Ok(format!("{} {forecast}", alerts_announcement(&alerts)))
}

/// Active National Weather Service alerts for a place, as text to speak.  `None` means none are
/// in effect and `only_if_any` was set.
pub fn alerts(place: &Location, only_if_any: bool) -> Result<Option<String>> {
    let nws = NwsClient::new()?;
    let alerts = nws.alerts(nws.locate(place)?.coordinates)?;
    if alerts.is_empty() && only_if_any {
        return Ok(None);
    }
    Ok(Some(alerts_announcement(&alerts)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_station_is_its_own_metar_station() {
        let place: Location = "egll".parse().unwrap();
        assert_eq!(metar_station(&place).unwrap(), "EGLL");
    }
}
