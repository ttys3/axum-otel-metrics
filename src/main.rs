mod middleware;

use axum::{response::Html, routing::get, Router, extract::State};
use std::net::SocketAddr;
use std::time::Instant;

use axum::{extract::MatchedPath, http::Request, response::IntoResponse};
use axum::middleware::{Next};
use opentelemetry::{Context, global};
use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry::sdk::export::metrics::aggregation;
use opentelemetry::sdk::metrics::{controllers, processors, selectors};

#[derive(Clone)]
pub struct SharedState {
    pub root_dir: String,
    pub metric: Metric,
}


#[derive(Clone)]
pub struct Metric {
    pub cx: Context,
    pub http_counter: Counter<u64>,

    // migrate from ValueRecorder to Histogram if opentelemetry 0.18.0 released
    pub http_req_histogram: Histogram<f64>,
}


#[tokio::main]
async fn main() {
    let meter = global::meter("my-app");

    let state = SharedState {
        root_dir: String::from("/tmp"),
        metric: Metric {
            cx: Default::default(),
            http_counter: meter.u64_counter("http.counter")
                .with_description("Counts http request")
                .init(),
            http_req_histogram:  meter.f64_histogram("http.histogram")
                .with_description("Counts http request latency")
                .init()
        }
    };

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

    // build our application with a route
    let app = Router::with_state(state.clone())
        .merge(crate::middleware::metrics::routes(exporter))
        .route("/", get(handler))
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), middleware::metrics::track_metrics));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}