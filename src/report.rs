//! The report a command announces, and the options that choose it.

use crate::{
    MetarData, Result, WeatherError, fetch_weather_data,
    nws::{Location, NwsClient},
    tts::{
        AnnouncementFormat, TemperatureUnit, alerts_announcement, forecast_announcement,
        generate_weather_announcement,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Report {
    /// Current conditions at an airport, from aviationweather.gov
    Metar,
    /// The National Weather Service forecast (US only)
    Forecast,
    /// Active National Weather Service alerts (US only)
    Alerts,
}

#[derive(clap::Args, Debug)]
pub struct ReportArgs {
    /// ICAO airport for metar (e.g., KSJC); station, lat,lon or US address for forecast and
    /// alerts
    pub location: String,

    /// What to announce
    #[arg(short, long, value_enum, default_value = "metar")]
    pub report: Report,

    /// Output format for the metar announcement
    #[arg(short, long, value_enum, default_value = "full")]
    pub format: AnnouncementFormat,

    /// Temperature unit for metar (the detailed format always gives both)
    #[arg(short, long, value_enum, default_value = "fahrenheit")]
    pub temp_unit: TemperatureUnit,

    /// Number of half-day forecast periods [default: 2]
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..))]
    pub periods: Option<u8>,

    /// For alerts: announce nothing when no alerts are in effect
    #[arg(long)]
    pub only_if_alerts: bool,
}

pub const NOTHING_TO_ANNOUNCE: &str = "No weather alerts in effect; nothing to announce.";

/// Fetches the chosen report and writes it as text to speak.  `None` means there is nothing to
/// announce: no alerts are in effect and `--only-if-alerts` was given.
pub fn announcement(args: &ReportArgs) -> Result<Option<String>> {
    if args.periods.is_some() && args.report != Report::Forecast {
        return Err(WeatherError::Usage(
            "--periods is only for --report forecast".to_string(),
        ));
    }
    if args.only_if_alerts && args.report != Report::Alerts {
        return Err(WeatherError::Usage(
            "--only-if-alerts is only for --report alerts".to_string(),
        ));
    }
    eprintln!("Fetching weather for {}...\n", args.location);

    if args.report == Report::Metar {
        let metar = fetch_weather_data(&args.location)?;
        return Ok(Some(generate_weather_announcement(
            &metar,
            &args.format,
            args.temp_unit,
        )));
    }

    let location: Location = args.location.parse()?;
    let nws = NwsClient::new()?;
    let coordinates = nws.locate(&location)?.coordinates;
    if args.report == Report::Alerts {
        let alerts = nws.alerts(coordinates)?;
        if alerts.is_empty() && args.only_if_alerts {
            return Ok(None);
        }
        return Ok(Some(alerts_announcement(&alerts)));
    }
    let periods = nws.forecast(&nws.point(coordinates)?)?;
    let count = periods.len().min(args.periods.unwrap_or(2).into());
    Ok(Some(forecast_announcement(&periods[..count])))
}

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

/// The National Weather Service forecast for a place, as text to speak.
pub fn nws_forecast(place: &Location, periods: usize) -> Result<String> {
    let nws = NwsClient::new()?;
    let point = nws.point(nws.locate(place)?.coordinates)?;
    let forecast = nws.forecast(&point)?;
    Ok(forecast_announcement(
        &forecast[..periods.min(forecast.len())],
    ))
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
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        report: ReportArgs,
    }

    fn parse(args: &[&str]) -> std::result::Result<ReportArgs, clap::Error> {
        Ok(Cli::try_parse_from(["test"].iter().chain(args))?.report)
    }

    #[test]
    fn one_report_is_chosen_and_metar_is_the_default() {
        assert_eq!(parse(&["KSJC"]).unwrap().report, Report::Metar);

        let args = parse(&[
            "-r",
            "forecast",
            "--periods",
            "4",
            "200 E Santa Clara St, San Jose, CA",
        ])
        .unwrap();
        assert_eq!(args.report, Report::Forecast);
        assert_eq!(args.periods, Some(4));
        assert_eq!(args.location, "200 E Santa Clara St, San Jose, CA");

        assert!(parse(&["-r", "taf", "KSJC"]).is_err());
        assert!(parse(&["-r", "forecast", "--periods", "0", "KSJC"]).is_err());
        assert!(parse(&["-t", "kelvin", "KSJC"]).is_err());
    }

    #[test]
    fn periods_are_only_for_the_forecast() {
        for report in ["metar", "alerts"] {
            let args = parse(&["-r", report, "--periods", "3", "KSJC"]).unwrap();
            assert!(matches!(announcement(&args), Err(WeatherError::Usage(_))));
        }
    }

    #[test]
    fn only_if_alerts_is_only_for_alerts() {
        for report in ["metar", "forecast"] {
            let args = parse(&["-r", report, "--only-if-alerts", "KSJC"]).unwrap();
            assert!(matches!(announcement(&args), Err(WeatherError::Usage(_))));
        }
    }
}
