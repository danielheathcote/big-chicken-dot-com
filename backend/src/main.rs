//! AWS Lambda entrypoint: an axum `Router` served through `lambda_http`.
//!
//! Exposes `GET /forecast`, which fetches the ECMWF ensemble forecast for
//! Cambridge from Open-Meteo and returns wind (median) + rain (p25/p50/p75).

mod handler;
mod open_meteo;
mod stats;

use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use lambda_http::{run, Error};

#[derive(Clone)]
struct AppState {
    http: reqwest::Client,
}

async fn forecast(State(state): State<AppState>) -> Result<Json<handler::Forecast>, (StatusCode, String)> {
    let resp = open_meteo::fetch(&state.http).await.map_err(|e| {
        tracing::error!(error = %e, "open-meteo request failed");
        (StatusCode::BAD_GATEWAY, format!("upstream error: {e}"))
    })?;
    Ok(Json(handler::build(resp)))
}

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // JSON-friendly logs to CloudWatch; RUST_LOG controls verbosity.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .without_time() // CloudWatch already timestamps each line.
        .init();

    let state = AppState {
        http: reqwest::Client::builder()
            .user_agent("big-chicken-forecast/0.1")
            .build()?,
    };

    let app = Router::new()
        .route("/", get(health))
        .route("/forecast", get(forecast))
        .with_state(state);

    run(app).await
}
