use axum::{extract::MatchedPath, http::Request, response::IntoResponse, routing::get, Router};
use axum::middleware::{FromFnLayer, Next};
use std::{
    time::{Instant},
};
use std::convert::Infallible;
use std::future::{Future, ready};
use axum::extract::State;
use axum::http::Response;
use axum::routing::Route;
use axum_core::Error;

use opentelemetry_prometheus::PrometheusExporter;

use prometheus::{Encoder, TextEncoder};

use opentelemetry::{
    KeyValue,Key,Value,
};
use axum_macros::debug_handler;

use opentelemetry::{Context, global};
use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry::sdk::export::metrics::aggregation;
use opentelemetry::sdk::metrics::{controllers, processors, selectors};

pub struct PromMetrics {
    pub(crate) state: MetricState,
}

#[derive(Clone)]
pub struct MetricState {
    exporter: PrometheusExporter,
    pub metric: Metric,
}

#[derive(Clone)]
pub struct Metric {
    pub cx: Context,
    pub http_counter: Counter<u64>,

    // migrate from ValueRecorder to Histogram if opentelemetry 0.18.0 released
    pub http_req_histogram: Histogram<f64>,
}

impl PromMetrics
{
    pub fn new() -> Self {

        let meter = global::meter("my-app");

        // init global meter provider and prometheus exporter
        let controller = controllers::basic(
            processors::factory(
                selectors::simple::histogram([0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
                aggregation::cumulative_temporality_selector(),
            )
                .with_memory(true),
        )
            .build();
        let exporter = opentelemetry_prometheus::exporter(controller).
            init();

        let app_state = MetricState {
            exporter,
            metric: Metric {
            cx: Default::default(),
            http_counter: meter.u64_counter("http.counter")
                .with_description("Counts http request")
                .init(),
            http_req_histogram:  meter.f64_histogram("http.histogram")
                .with_description("Counts http request latency")
                .init()
        } };

        Self { state: app_state }
    }


    pub fn routes(&self) -> Router<MetricState> {
        Router::with_state(self.state.clone())
            .route("/metrics", get(exporter_handler))
    }

    // fn track_middleware<L>(self) -> FromFnLayer<F, S, T>
    // {
    //     axum::middleware::from_fn_with_state(self.state.clone(), track_metrics)
    // }
}

#[debug_handler]
pub async fn exporter_handler(state: State<MetricState>) -> impl IntoResponse {
    println!("metrics api");
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    encoder.encode(&state.exporter.registry().gather(), &mut buffer).unwrap();
    let metrics = String::from_utf8(buffer).unwrap();
    metrics
}


// record handler metrics
pub async fn track_metrics<B>(state: State<MetricState>, req: Request<B>, next: Next<B>) -> axum::response::Response {
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

    tracing::info!("{}", method);
    println!("ok");

    response
}