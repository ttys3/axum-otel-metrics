mod middleware;

use axum::{extract::State, response::Html, routing::get, Router};
use std::net::SocketAddr;
use std::time::Instant;

use crate::middleware::metrics::{PromMetrics, PromMetricsLayer};
use axum::middleware::Next;
use axum::{extract::MatchedPath, http::Request, response::IntoResponse};
use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry::sdk::export::metrics::aggregation;
use opentelemetry::sdk::metrics::{controllers, processors, selectors};
use opentelemetry::{global, Context};
use tower::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct SharedState {
    pub root_dir: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "axum_metrics_demo=debug,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = SharedState {
        root_dir: String::from("/tmp"),
    };

    let metrics = PromMetricsLayer::new();

    // build our application with a route
    let app = Router::with_state(state.clone())
        .merge(metrics.routes())
        .route("/", get(handler))
        .route("/hello", get(handler))
        .route_layer(metrics);

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
