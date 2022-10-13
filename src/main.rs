// #![feature(generic_associated_types)]
// #![feature(type_alias_impl_trait)]

mod middleware;

use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;
use std::time;
use axum::extract::State;

use crate::middleware::metrics::PromMetricsLayer;
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

async fn handler(state: State<SharedState>) -> Html<String> {
    std::thread::sleep(time::Duration::from_millis(800));
    Html(format!("<h1>Hello, World!</h1> root_dir={}", state.root_dir))
}
