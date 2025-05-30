use axum::extract::{MatchedPath, State};
use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use rand::Rng;
use std::time;

use axum::response::Response;
use axum_otel_metrics::{HttpMetricsLayerBuilder, PathSkipper};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use opentelemetry::metrics::Counter;
use opentelemetry::{global, KeyValue};
use opentelemetry_semantic_conventions::attribute::{DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_VERSION};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};

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

    println!("Using OTLP metrics exporter to send metrics to OpenTelemetry collector");

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_temporality(Temporality::default())
        .build()
        .unwrap();

    let reader = PeriodicReader::builder(exporter)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(env!("CARGO_PKG_NAME").to_string())
        .with_attributes(vec![
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION").to_string()),
            KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "dev"),
        ])
        .build();

    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();
    // TODO: ensure defer run `provider.shutdown()?;`

    global::set_meter_provider(provider.clone());

    let metrics = HttpMetricsLayerBuilder::new()
        .with_skipper(PathSkipper::new(|s| s.starts_with("/skip")))
        // from 5 bytes to 500 bytes
        .with_size_buckets(vec![5.0, 20.0, 50.0, 100.0, 500.0])
        // from 1ms to 100ms
        .with_duration_buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1])
        .build();

    let state = SharedState {
        root_dir: String::from("/tmp"),
        foobar: global::meter("axum-app").u64_counter("foobar").build(),
    };

    // build our application with a route
    let app = Router::new()
        .nest("/sub", crate::sub::routes())
        .route("/", get(handler))
        .route("/hello", get(handler))
        .route("/world", get(handler))
        .route("/skip-this", get(handler))
        .route("/post", post(handler))
        .layer(metrics)
        .layer(axum::middleware::map_response(set_header))
        .fallback(|| async { Html("404 page not found".to_string()) })
        .with_state(state.clone());

    // run it
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn set_header<B>(mut response: Response<B>) -> Response<B> {
    response.headers_mut().insert("x-test-key", "foo".parse().unwrap());
    response
}

async fn handler(state: State<SharedState>, path: MatchedPath) -> Html<String> {
    let mut rng = rand::rng();

    // Default size covers all buckets (5-500 bytes)
    let n_bytes = rng.random_range(1..=500);
    let mut dummy = ".".repeat(n_bytes);

    let delay_ms: u64;
    match path.as_str() {
        "/hello" => {
            // Small responses (1-20 bytes) covering first two buckets
            let n_bytes = rng.random_range(1..=20);
            dummy = ".".repeat(n_bytes);
            // Quick responses (0-10ms) covering first three duration buckets
            delay_ms = rng.random_range(0..=10);
            std::thread::sleep(time::Duration::from_millis(delay_ms))
        }
        "/world" => {
            // Medium responses (20-100 bytes) covering middle buckets
            let n_bytes = rng.random_range(20..=100);
            dummy = ".".repeat(n_bytes);
            // Medium latency (10-50ms) covering middle duration buckets
            delay_ms = rng.random_range(10..=50);
            std::thread::sleep(time::Duration::from_millis(delay_ms))
        }
        _ => {
            // Larger responses and higher latency for default route
            // covering all buckets including the highest ones
            delay_ms = rng.random_range(50..=100);
            std::thread::sleep(time::Duration::from_millis(delay_ms));
        }
    }

    state.foobar.add(1, &[KeyValue::new("attr1", "foo")]);

    Html(format!(
    "\n\n<h1>Request path: {}</h1> <hr />\nroot_dir={}\nsleep_ms={}\n\
    <hr /><a href='/'>/</a>\n\
    <a href='/hello'>/hello</a>\n\
    <a href='/world'>/world</a>\n\
    <a href='/sub/sub1'>/sub/sub1</a>\n\
    <a href='/sub/sub2'>/sub/sub2</a>\n\
    <a href='/skip-this'>/skip-this</a>\n\
    <hr /> {} \n\n",
        path.as_str(),
        state.root_dir,
        delay_ms,
        dummy,
    ))
}
