use axum::extract::{MatchedPath, State};
use axum::{response::Html, routing::{get, post}, Router};
use rand::Rng;
use std::time;

use axum_otel_metrics::{HttpMetricsLayerBuilder, PathSkipper};
use axum::response::Response;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::metrics::{Counter};
use opentelemetry::{global, KeyValue};

mod sub;

#[derive(Clone)]
pub struct SharedState {
    pub root_dir: String,
    foobar: Counter<u64>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axum_metrics_demo=debug,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let metrics = HttpMetricsLayerBuilder::new()
        .with_service_name(env!("CARGO_PKG_NAME").to_string())
        .with_service_version(env!("CARGO_PKG_VERSION").to_string())
        // .with_prefix("axum_metrics_demo".to_string())
        .with_labels(vec![("env".to_string(), "dev".to_string())].into_iter().collect())
        .with_skipper(PathSkipper::new(|s| s.starts_with("/skip")))
        .build();

    let state = SharedState {
        root_dir: String::from("/tmp"),
        foobar: global::meter("axum-app").u64_counter("foobar").init(),
    };


    // build our application with a route
    let app = Router::new()
        .merge(metrics.routes::<SharedState>())
        .nest("/sub", crate::sub::routes())
        .route("/", get(handler))
        .route("/hello", get(handler))
        .route("/world", get(handler))
        .route("/skip-this", get(handler))
        .route("/post", post(handler))
        .layer(metrics)
        .layer(axum::middleware::map_response(set_header))
        .fallback(||{
            async { Html("404 page not found".to_string()) }
        })
        .with_state(state.clone());

    // run it
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();
    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn set_header<B>(mut response: Response<B>) -> Response<B> {
    response.headers_mut().insert("x-test-key", "foo".parse().unwrap());
    response
}

async fn handler(state: State<SharedState>, path: MatchedPath) -> Html<String> {
    let mut rng = rand::thread_rng();

    // 1024 / 16 = 64
    let n_bytes = rng.gen_range(1..5120*64); // 16 bytes to 5120 KB
    let mut dummy = "123456789abcdef".repeat(n_bytes);

    let delay_ms: u64;
    match path.as_str() {
        "/hello" => {
            delay_ms = rng.gen_range(0..300);
            std::thread::sleep(time::Duration::from_millis(delay_ms))
        }
        "/world" => {
            delay_ms = rng.gen_range(0..500);
            std::thread::sleep(time::Duration::from_millis(delay_ms))
        }
        _ => {
            delay_ms = rng.gen_range(0..100);
            std::thread::sleep(time::Duration::from_millis(delay_ms));
            dummy = "".to_string();
        }
    }

    state.foobar.add(1, &[KeyValue::new("attr1", "foo")]);

    Html(format!(
        "<h1>Request path: {}</h1> <hr />\nroot_dir={}\nsleep_ms={}\n\
    <hr /><a href='/' style='display: inline-block; width: 100px;'>/</a>\n\
    <a href='/hello' style='display: inline-block; width: 100px;'>/hello</a>\n\
    <a href='/world' style='display: inline-block; width: 100px;'>/world</a>\n\
    <a href='/sub/sub1' style='display: inline-block; width: 100px;'>/sub/sub1</a>\n\
    <a href='/sub/sub2' style='display: inline-block; width: 100px;'>/sub/sub2</a>\n\
    <a href='/skip-this' style='display: inline-block; width: 100px;'>/skip-this</a>\n\
    <hr /><a href='/metrics'>/metrics</a>{}\n\n",
        path.as_str(),
        state.root_dir,
        delay_ms,
        dummy,
    ))
}
