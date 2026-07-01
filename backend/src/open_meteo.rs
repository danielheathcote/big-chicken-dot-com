//! Client for the Open-Meteo Ensemble API (ECMWF IFS ensemble).
//!
//! The ensemble endpoint returns, for each requested variable, a bare control
//! series (e.g. `wind_speed_10m`) plus one series per perturbed member
//! (`wind_speed_10m_member01` .. `wind_speed_10m_member50`) — 51 members total.
//! We capture them all via a flattened map and select by name prefix, so the
//! member count is never hard-coded.

use std::collections::HashMap;

use serde::Deserialize;

/// Cambridge, UK.
pub const LATITUDE: f64 = 52.2053;
pub const LONGITUDE: f64 = 0.1218;
pub const LOCATION_NAME: &str = "Cambridge";

const ENDPOINT: &str = "https://ensemble-api.open-meteo.com/v1/ensemble";

/// Number of forecast days requested from the upstream API.
pub const FORECAST_DAYS: u8 = 5;

#[derive(Debug, Deserialize)]
pub struct EnsembleResponse {
    pub hourly: Hourly,
}

#[derive(Debug, Deserialize)]
pub struct Hourly {
    pub time: Vec<String>,
    /// Every remaining hourly array, keyed by its variable name. Values may be
    /// `null` at the edges of a member's coverage, hence `Option<f64>`.
    #[serde(flatten)]
    pub series: HashMap<String, Vec<Option<f64>>>,
}

impl Hourly {
    /// All member series for a variable: the bare control key plus every
    /// `"{var}_member.."` key. Returns one inner vec per member.
    pub fn members(&self, var: &str) -> Vec<&Vec<Option<f64>>> {
        let member_prefix = format!("{var}_member");
        self.series
            .iter()
            .filter(|(k, _)| k.as_str() == var || k.starts_with(&member_prefix))
            .map(|(_, v)| v)
            .collect()
    }
}

/// Fetch the ECMWF ensemble forecast for Cambridge.
pub async fn fetch(client: &reqwest::Client) -> Result<EnsembleResponse, reqwest::Error> {
    client
        .get(ENDPOINT)
        .query(&[
            ("latitude", LATITUDE.to_string()),
            ("longitude", LONGITUDE.to_string()),
            ("hourly", "wind_speed_10m,precipitation".to_string()),
            ("models", "ecmwf_ifs025".to_string()),
            ("forecast_days", FORECAST_DAYS.to_string()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<EnsembleResponse>()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // A trimmed response: 2 hours, control + 2 members for each variable.
    const SAMPLE: &str = r#"{
      "hourly": {
        "time": ["2026-07-01T00:00", "2026-07-01T01:00"],
        "wind_speed_10m": [10.0, 11.0],
        "wind_speed_10m_member01": [12.0, 13.0],
        "wind_speed_10m_member02": [8.0, 9.0],
        "precipitation": [0.0, 0.5],
        "precipitation_member01": [0.1, 1.0],
        "precipitation_member02": [0.0, null]
      }
    }"#;

    #[test]
    fn parses_and_groups_members() {
        let resp: EnsembleResponse = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(resp.hourly.time.len(), 2);

        let wind = resp.hourly.members("wind_speed_10m");
        assert_eq!(wind.len(), 3, "control + 2 members");

        let rain = resp.hourly.members("precipitation");
        assert_eq!(rain.len(), 3);

        // The precipitation prefix must not accidentally swallow other vars.
        assert!(resp.hourly.members("wind_speed_10m").iter().all(|s| s.len() == 2));
    }
}
