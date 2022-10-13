// #![feature(generic_associated_types)]
// #![feature(type_alias_impl_trait)]

mod middleware;

use axum::extract::{MatchedPath, State};
use axum::{response::Html, routing::get, Router};
use rand::Rng;
use std::net::SocketAddr;
use std::time;

use crate::middleware::metrics::PromMetricsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct SharedState {
    pub root_dir: String,
    // pub rnd: rand::thread_rng(),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_metrics_demo=debug,tower_http=info".into()),
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
        .route("/world", get(handler))
        .route_layer(metrics);

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

async fn handler(state: State<SharedState>, path: MatchedPath) -> Html<String> {
    let mut rng = rand::thread_rng();
    let delay_ms: u64 = rng.gen::<u64>() % 800;
    std::thread::sleep(time::Duration::from_millis(delay_ms));
    Html(format!(
        "<h1>Request path: {}</h1> <hr />\nroot_dir={}\nsleep_ms={}\n\
    <hr /><a href='/' style='display: inline-block; width: 100px;'>/</a>\n\
    <a href='/hello' style='display: inline-block; width: 100px;'>/hello</a>\n\
    <a href='/world' style='display: inline-block; width: 100px;'>/world</a>\n\
    <hr /><a href='/metrics'>/metrics</a>\n\n",
        path.as_str(),
        state.root_dir,
        delay_ms
    ))
}
