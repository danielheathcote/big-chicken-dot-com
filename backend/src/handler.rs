//! Builds the `/forecast` JSON payload from the raw ensemble response.

use serde::Serialize;

use crate::open_meteo::{self, EnsembleResponse};
use crate::stats::{percentile, sorted_finite};

#[derive(Debug, Serialize)]
pub struct Forecast {
    pub location: Location,
    pub generated_at: String,
    pub hourly: HourlyOut,
}

#[derive(Debug, Serialize)]
pub struct Location {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Serialize)]
pub struct HourlyOut {
    pub time: Vec<String>,
    /// Ensemble median wind speed (km/h) per hour — the line graph series.
    pub wind_speed_10m: Vec<Option<f64>>,
    /// Rain (mm/h) percentiles across members — the box-plot series.
    pub rain: RainPercentiles,
}

#[derive(Debug, Serialize)]
pub struct RainPercentiles {
    pub p25: Vec<Option<f64>>,
    pub p50: Vec<Option<f64>>,
    pub p75: Vec<Option<f64>>,
}

/// Collect the value at hour `i` from every member series, sorted and finite.
fn column(members: &[&Vec<Option<f64>>], i: usize) -> Vec<f64> {
    sorted_finite(members.iter().map(|series| series.get(i).copied().flatten()))
}

/// Transform the upstream ensemble response into the public forecast payload.
pub fn build(resp: EnsembleResponse) -> Forecast {
    let hourly = &resp.hourly;
    let n = hourly.time.len();

    let wind_members = hourly.members("wind_speed_10m");
    let rain_members = hourly.members("precipitation");

    let mut wind_speed_10m = Vec::with_capacity(n);
    let mut p25 = Vec::with_capacity(n);
    let mut p50 = Vec::with_capacity(n);
    let mut p75 = Vec::with_capacity(n);

    for i in 0..n {
        let wind = column(&wind_members, i);
        wind_speed_10m.push(percentile(&wind, 0.50));

        let rain = column(&rain_members, i);
        p25.push(percentile(&rain, 0.25));
        p50.push(percentile(&rain, 0.50));
        p75.push(percentile(&rain, 0.75));
    }

    Forecast {
        location: Location {
            name: open_meteo::LOCATION_NAME.to_string(),
            lat: open_meteo::LATITUDE,
            lon: open_meteo::LONGITUDE,
        },
        generated_at: chrono::Utc::now().to_rfc3339(),
        hourly: HourlyOut {
            time: hourly.time.clone(),
            wind_speed_10m,
            rain: RainPercentiles { p25, p50, p75 },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "hourly": {
        "time": ["2026-07-01T00:00", "2026-07-01T01:00"],
        "wind_speed_10m": [10.0, 11.0],
        "wind_speed_10m_member01": [12.0, 13.0],
        "wind_speed_10m_member02": [8.0, 9.0],
        "precipitation": [0.0, 0.5],
        "precipitation_member01": [0.1, 1.0],
        "precipitation_member02": [0.2, null]
      }
    }"#;

    #[test]
    fn builds_medians_and_percentiles() {
        let resp: EnsembleResponse = serde_json::from_str(SAMPLE).unwrap();
        let f = build(resp);

        assert_eq!(f.hourly.time.len(), 2);
        // Hour 0 wind members {8,10,12} -> median 10.
        assert_eq!(f.hourly.wind_speed_10m[0], Some(10.0));
        // Hour 0 rain members {0.0,0.1,0.2} -> p50 0.1.
        assert_eq!(f.hourly.rain.p50[0], Some(0.1));

        // Percentiles stay ordered where present.
        for i in 0..2 {
            if let (Some(a), Some(b), Some(c)) =
                (f.hourly.rain.p25[i], f.hourly.rain.p50[i], f.hourly.rain.p75[i])
            {
                assert!(a <= b && b <= c);
            }
        }
    }
}
