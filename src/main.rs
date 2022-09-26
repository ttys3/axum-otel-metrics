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
use crate::middleware::metrics::{PromMetrics, track_metrics};

#[derive(Clone)]
pub struct SharedState {
    pub root_dir: String,
}


#[tokio::main]
async fn main() {
    let state = SharedState {
        root_dir: String::from("/tmp"),
    };

    let metrics = PromMetrics::new();

    // build our application with a route
    let app = Router::with_state(state.clone())
        .merge(metrics.routes())
        .route("/", get(handler))
        .route_layer(axum::middleware::from_fn_with_state(metrics.state.clone(), track_metrics));

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