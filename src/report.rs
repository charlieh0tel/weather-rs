//! The report a command announces, and the options that choose it.

use crate::{
    Result, WeatherError, fetch_weather_data,
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
    #[arg(short, long, value_enum, default_value = "speech")]
    pub format: AnnouncementFormat,

    /// Temperature unit for metar (the detailed format always gives both)
    #[arg(short, long, value_enum, default_value = "fahrenheit")]
    pub temp_unit: TemperatureUnit,

    /// Number of half-day forecast periods [default: 2]
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..))]
    pub periods: Option<u8>,
}

/// Fetches the chosen report and writes it as text to speak.
pub fn announcement(args: &ReportArgs) -> Result<String> {
    if args.periods.is_some() && args.report != Report::Forecast {
        return Err(WeatherError::Usage(
            "--periods is only for --report forecast".to_string(),
        ));
    }
    println!("Fetching weather for {}...\n", args.location);

    if args.report == Report::Metar {
        let metar = fetch_weather_data(&args.location)?;
        return Ok(generate_weather_announcement(
            &metar,
            &args.format,
            args.temp_unit,
        ));
    }

    let location: Location = args.location.parse()?;
    let nws = NwsClient::new()?;
    let coordinates = nws.locate(&location)?.coordinates;
    if args.report == Report::Alerts {
        return Ok(alerts_announcement(&nws.alerts(coordinates)?));
    }
    let periods = nws.forecast(&nws.point(coordinates)?)?;
    let count = periods.len().min(args.periods.unwrap_or(2).into());
    Ok(forecast_announcement(&periods[..count]))
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
}
