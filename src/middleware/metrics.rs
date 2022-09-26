use axum::{extract::MatchedPath, http::Request, response::IntoResponse, routing::get, Router};
use axum::middleware::{Next};
use std::{
    time::{Instant},
};
use std::future::ready;
use axum::extract::State;

use opentelemetry_prometheus::PrometheusExporter;

use prometheus::{Encoder, TextEncoder};

use opentelemetry::{
    KeyValue,Key,Value,
};
use axum_macros::debug_handler;

#[derive(Clone)]
pub struct AppState {
    exporter: PrometheusExporter,
}

pub fn routes(exporter: PrometheusExporter) -> Router<AppState> {
    let app_state = AppState { exporter };
    Router::with_state(app_state)
        .route("/metrics", get(exporter_handler))
}

#[debug_handler]
pub async fn exporter_handler(state: State<AppState>) -> impl IntoResponse {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&state.exporter.registry().gather(), &mut buffer).unwrap();
    let metrics = String::from_utf8(buffer).unwrap();
    metrics
}

// warning: the state is not an extractor!
pub async fn track_metrics<B>(state: State<crate::SharedState>, req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };

    let method = req.method().clone();

    let response = next.run(req).await;

    if path == "/metrics" {
        return response;
    }

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        KeyValue{key: Key::from("method"), value: Value::from(method.to_string()) },
        KeyValue::new("path", path),
        KeyValue::new("status", status),
    ];

    state.metric.http_counter.add(&state.metric.cx, 1, &labels);

    state.metric.http_req_histogram.record(&state.metric.cx, latency, &labels);

    response
}