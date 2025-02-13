//! [axum](https://github.com/tokio-rs/axum) OpenTelemetry Metrics middleware
//!
//! ## Simple Usage: with otlp exporter
//!
//! Meter provider should be configured through [opentelemetry_sdk `global::set_meter_provider`](https://docs.rs/opentelemetry/0.28.0/opentelemetry/global/index.html#global-metrics-api).
//! if you want to use the [prometheus exporter](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/prometheus/), see [Advanced Usage](#advanced-usage) below.
//!
//! ```
//! use axum_otel_metrics::HttpMetricsLayerBuilder;
//! use axum::{response::Html, routing::get, Router};
//! use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
//! use opentelemetry::global;
//! 
//! let exporter = opentelemetry_otlp::MetricExporter::builder()
//!     .with_http()
//!     .with_temporality(Temporality::default())
//!     .build()
//!     .unwrap();
//!
//! let reader = PeriodicReader::builder(exporter)
//!     .with_interval(std::time::Duration::from_secs(30))
//!     .build();
//!
//! let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
//!     .with_reader(reader)
//!     .build();
//!
//!  // TODO: ensure defer run `provider.shutdown()?;`
//!
//! global::set_meter_provider(provider.clone());
//!
//! let metrics = HttpMetricsLayerBuilder::new()
//!     .build();
//!
//! let app = Router::<()>::new()
//!     .route("/", get(handler))
//!     .route("/hello", get(handler))
//!     .route("/world", get(handler))
//!     // add the metrics middleware
//!     .layer(metrics);
//!
//! async fn handler() -> Html<&'static str> {
//!     Html("<h1>Hello, World!</h1>")
//! }
//! ```
//!
//! ## Advanced Usage: with prometheus exporter
//!
//! this is an example to use the [prometheus exporter](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/prometheus/)
//!
//! it will export the metrics at `/metrics` endpoint
//!
//! ```
//! use axum_otel_metrics::HttpMetricsLayerBuilder;
//! use axum::{response::Html, routing::get, Router};
//!
//! use opentelemetry::global;
//! use opentelemetry_sdk::metrics::SdkMeterProvider;
//! use prometheus::{Encoder, Registry, TextEncoder};
//!
//! let exporter = opentelemetry_prometheus::exporter()
//!     .with_registry(prometheus::default_registry().clone())
//!     .build()
//!     .unwrap();
//!
//! let provider = SdkMeterProvider::builder().with_reader(exporter).build();
//! // TODO: ensure defer run `provider.shutdown()?;`
//!
//! global::set_meter_provider(provider.clone());
//!
//! let metrics = HttpMetricsLayerBuilder::new().build();
//!
//! let app = Router::<()>::new()
//!     // export metrics at `/metrics` endpoint
//!     .route("/metrics", get(|| async {
//!         let mut buffer = Vec::new();
//!         let encoder = TextEncoder::new();
//!         encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
//!         // return metrics
//!         String::from_utf8(buffer).unwrap()
//!     }))
//!     .route("/", get(handler))
//!     .route("/hello", get(handler))
//!     .route("/world", get(handler))
//!     // add the metrics middleware
//!     .layer(metrics);
//!
//! async fn handler() -> Html<&'static str> {
//!     Html("<h1>Hello, World!</h1>")
//! }
//! ```

use axum::http::Response;
use axum::{extract::MatchedPath, http, http::Request};
use std::env;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll::Ready;
use std::task::{Context, Poll};
use std::time::Instant;

use opentelemetry::global;
use opentelemetry::metrics::{Histogram, UpDownCounter};
use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::metric::{
    HTTP_SERVER_ACTIVE_REQUESTS, HTTP_SERVER_REQUEST_BODY_SIZE, HTTP_SERVER_REQUEST_DURATION, HTTP_SERVER_RESPONSE_BODY_SIZE,
};

use tower::{Layer, Service};

use futures_util::ready;
use http_body::Body as httpBody;
use pin_project_lite::pin_project; // for `Body::size_hint`

/// the metrics we used in the middleware
#[derive(Clone)]
pub struct Metric {
    pub req_duration: Histogram<f64>,

    pub req_body_size: Histogram<u64>,

    pub res_body_size: Histogram<u64>,

    pub req_active: UpDownCounter<i64>,
}

#[derive(Clone)]
pub struct MetricState {
    /// hold the metrics we used in the middleware
    pub metric: Metric,

    /// PathSkipper used to skip some paths for not recording metrics
    skipper: PathSkipper,

    /// whether the service is running as a TLS server or not.
    /// this is used to help determine the `url.scheme` otel meter attribute.
    /// because there is no way to get the scheme from the request in http server
    /// (except for absolute uri request, but which is only used when as a proxy server).
    is_tls: bool,
}

/// the service wrapper
#[derive(Clone)]
pub struct HttpMetrics<S> {
    pub(crate) state: MetricState,

    /// inner service which is wrapped by this middleware
    service: S,
}

#[derive(Clone)]
pub struct HttpMetricsLayer {
    /// the metric state, use both by the middleware handler and metrics export endpoint
    pub(crate) state: MetricState,
}

// TODO support custom buckets
// allocation not allowed in statics: static HTTP_REQ_DURATION_HISTOGRAM_BUCKETS: Vec<f64> = vec![0, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5, 7.5, 10];
// as https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md#metric-httpserverrequestduration spec
// This metric SHOULD be specified with ExplicitBucketBoundaries of [ 0, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5, 7.5, 10 ].
// the unit of the buckets is second
const HTTP_REQ_DURATION_HISTOGRAM_BUCKETS: &[f64] = &[
    0.0, 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];

const KB: f64 = 1024.0;
const MB: f64 = 1024.0 * KB;

const HTTP_REQ_SIZE_HISTOGRAM_BUCKETS: &[f64] = &[
    1.0 * KB,   // 1 KB
    2.0 * KB,   // 2 KB
    5.0 * KB,   // 5 KB
    10.0 * KB,  // 10 KB
    100.0 * KB, // 100 KB
    500.0 * KB, // 500 KB
    1.0 * MB,   // 1 MB
    2.5 * MB,   // 2 MB
    5.0 * MB,   // 5 MB
    10.0 * MB,  // 10 MB
];

/// A helper that instructs the metrics layer to ignore
/// certain paths.
///
/// The [HttpMetricsLayerBuilder] uses this helper during the
/// construction of the [HttpMetricsLayer] that will be called
/// by Axum / Hyper / Tower when a request comes in.
#[derive(Clone)]
pub struct PathSkipper {
    skip: Arc<dyn Fn(&str) -> bool + 'static + Send + Sync>,
}

impl PathSkipper {
    /// Returns a [PathSkipper] that skips recording metrics
    /// for requests whose path, when passed to `fn`, returns
    /// `true`.
    ///
    /// Only static functions are accepted -- callables such
    /// as closures that capture their surrounding context will
    /// not work here.  For a variant that works, consult the
    /// [PathSkipper::new_with_fn] method.
    pub fn new(skip: fn(&str) -> bool) -> Self {
        Self { skip: Arc::new(skip) }
    }

    /// Dynamic variant of [PathSkipper::new].
    ///
    /// This variant requires the callable to be wrapped in an
    /// [Arc] but, in exchange for this requirement, the caller
    /// can use closures that capture variables from their context.
    ///
    /// The callable argument *must be thread-safe*.  You, as
    /// the implementor and user of this code, have that
    /// responsibility.
    pub fn new_with_fn(skip: Arc<dyn Fn(&str) -> bool + 'static + Send + Sync>) -> Self {
        Self { skip }
    }
}

impl Default for PathSkipper {
    /// Returns a `PathSkipper` that skips any path which
    /// starts with `/metrics` or `/favicon.ico``.
    ///
    /// This is the default implementation used when
    /// building an HttpMetricsLayerBuilder from scratch.
    fn default() -> Self {
        Self::new(|s| s.starts_with("/metrics") || s.starts_with("/favicon.ico"))
    }
}

#[derive(Clone, Default)]
pub struct HttpMetricsLayerBuilder {
    skipper: PathSkipper,
    is_tls: bool,
    duration_buckets: Option<Vec<f64>>,
    size_buckets: Option<Vec<f64>>,
    provider: Option<Arc<dyn opentelemetry::metrics::MeterProvider + Send + Sync>>,
}

impl HttpMetricsLayerBuilder {
    pub fn new() -> Self {
        HttpMetricsLayerBuilder::default()
    }

    pub fn with_skipper(mut self, skipper: PathSkipper) -> Self {
        self.skipper = skipper;
        self
    }

    pub fn with_duration_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.duration_buckets = Some(buckets);
        self
    }

    pub fn with_size_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.size_buckets = Some(buckets);
        self
    }

    pub fn with_provider<P>(mut self, provider: P) -> Self
    where
        P: opentelemetry::metrics::MeterProvider + Send + Sync + 'static,
    {
        self.provider = Some(Arc::new(provider));
        self
    }

    pub fn build(self) -> HttpMetricsLayer {
        let provider = self.provider.unwrap_or_else(|| {
            global::meter_provider()
        });

        let meter = provider.meter_with_scope(
            opentelemetry::InstrumentationScope::builder(env!("CARGO_PKG_NAME"))
                .with_version(env!("CARGO_PKG_VERSION"))
                .build(),
        );

        let duration_buckets = self
            .duration_buckets
            .unwrap_or_else(|| HTTP_REQ_DURATION_HISTOGRAM_BUCKETS.to_vec());

        let size_buckets = self.size_buckets.unwrap_or_else(|| HTTP_REQ_SIZE_HISTOGRAM_BUCKETS.to_vec());

        let req_duration = meter
            .f64_histogram(HTTP_SERVER_REQUEST_DURATION)
            .with_unit("s")
            .with_description("The HTTP request latencies in seconds.")
            .with_boundaries(duration_buckets)
            .build();

        let req_size = meter
            .u64_histogram(HTTP_SERVER_REQUEST_BODY_SIZE)
            .with_unit("By")
            .with_description("The HTTP request sizes in bytes.")
            .with_boundaries(size_buckets.clone())
            .build();

        let res_size = meter
            .u64_histogram(HTTP_SERVER_RESPONSE_BODY_SIZE)
            .with_unit("By")
            .with_description("The HTTP response sizes in bytes.")
            .with_boundaries(size_buckets)
            .build();

        // no u64_up_down_counter because up_down_counter maybe < 0 since it allow negative values
        let req_active = meter
            .i64_up_down_counter(HTTP_SERVER_ACTIVE_REQUESTS)
            .with_description("The number of active HTTP requests.")
            .build();

        let meter_state = MetricState {
            metric: Metric {
                req_duration,
                req_body_size: req_size,
                res_body_size: res_size,
                req_active,
            },
            skipper: self.skipper,
            is_tls: self.is_tls,
        };

        HttpMetricsLayer { state: meter_state }
    }
}

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetrics<S>;

    fn layer(&self, service: S) -> Self::Service {
        HttpMetrics {
            state: self.state.clone(),
            service,
        }
    }
}

pin_project! {
    /// Response future for [`HttpMetrics`] Service.
    pub struct ResponseFuture<F> {
        #[pin]
        inner: F,
        start: Instant,
        state: MetricState,
        path: String,
        method: String,
        url_scheme: String,
        host: String,
        req_body_size: u64,
    }
}

impl<S, R, ResBody> Service<Request<R>> for HttpMetrics<S>
where
    S: Service<Request<R>, Response = Response<ResBody>>,
    ResBody: httpBody,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: Request<R>) -> Self::Future {
        let url_scheme = if self.state.is_tls {
            "https".to_string()
        } else {
            (|| {
                if let Some(scheme) = req.headers().get("X-Forwarded-Proto") {
                    return scheme.to_str().unwrap().to_string();
                } else if let Some(scheme) = req.headers().get("X-Forwarded-Protocol") {
                    return scheme.to_str().unwrap().to_string();
                }
                if req.headers().get("X-Forwarded-Ssl").is_some().to_string() == "on" {
                    return "https".to_string();
                }
                if let Some(scheme) = req.headers().get("X-Url-Scheme") {
                    scheme.to_str().unwrap().to_string()
                } else {
                    "http".to_string()
                }
            })()
        };
        // ref https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md#metric-httpserveractive_requests
        // http.request.method and url.scheme is required
        self.state.metric.req_active.add(
            1,
            &[
                KeyValue::new("http.request.method", req.method().as_str().to_string()),
                KeyValue::new("url.scheme", url_scheme.clone()),
            ],
        );
        let start = Instant::now();
        let method = req.method().clone().to_string();
        let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
            matched_path.as_str().to_owned()
        } else {
            "".to_owned()
        };

        let host = req
            .headers()
            .get(http::header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let req_body_size = compute_request_body_size(&req);

        // for scheme, see github.com/labstack/echo/v4@v4.11.1/context.go
        // we can not use req.uri().scheme() since for non-absolute uri, it is always None

        ResponseFuture {
            inner: self.service.call(req),
            start,
            method,
            path,
            host,
            req_body_size: req_body_size as u64,
            state: self.state.clone(),
            url_scheme,
        }
    }
}

/// compute approximate request size
///
/// the implementation refs [labstack/echo-contrib 's prometheus middleware](https://github.com/labstack/echo-contrib/blob/db8911a1af7abb6bdafbd999adada548fd9c0849/echoprometheus/prometheus.go#L329)
fn compute_approximate_request_size<T>(req: &Request<T>) -> usize {
    let mut s = 0;
    s += req.uri().path().len();
    s += req.method().as_str().len();

    req.headers().iter().for_each(|(k, v)| {
        s += k.as_str().len();
        s += v.as_bytes().len();
    });

    s += req.uri().host().map(|h| h.len()).unwrap_or(0);

    s += req
        .headers()
        .get(http::header::CONTENT_LENGTH)
        .map(|v| v.to_str().unwrap().parse::<usize>().unwrap_or(0))
        .unwrap_or(0);
    s
}

fn compute_request_body_size<T>(req: &Request<T>) -> usize {
    req.headers()
        .get(http::header::CONTENT_LENGTH)
        .map(|v| v.to_str().unwrap().parse::<usize>().unwrap_or(0))
        .unwrap_or(0)
}

impl<F, B: httpBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let response = ready!(this.inner.poll(cx))?;

        this.state.metric.req_active.add(
            -1,
            &[
                KeyValue::new("http.request.method", this.method.clone()),
                KeyValue::new("url.scheme", this.url_scheme.clone()),
            ],
        );

        if (this.state.skipper.skip)(this.path.as_str()) {
            return Poll::Ready(Ok(response));
        }

        let latency = this.start.elapsed().as_secs_f64();
        let status = response.status().as_u16().to_string();

        let res_body_size = response.body().size_hint().upper().unwrap_or(0);

        let labels = [
            KeyValue::new("http.request.method", this.method.clone()),
            KeyValue::new("http.route", this.path.clone()),
            KeyValue::new("http.response.status_code", status),
            // server.address: Name of the local HTTP server that received the request.
            // Determined by using the first of the following that applies
            //
            // 1. The primary server name of the matched virtual host. MUST only include host identifier.
            // 2. Host identifier of the request target if it's sent in absolute-form.
            // 3. Host identifier of the Host header
            KeyValue::new("server.address", this.host.clone()),
        ];
        this.state.metric.req_body_size.record(*this.req_body_size, &labels);

        this.state.metric.res_body_size.record(res_body_size, &labels);

        this.state.metric.req_duration.record(latency, &labels);

        Ready(Ok(response))
    }
}

#[cfg(test)]
mod tests {
    use crate::HttpMetricsLayerBuilder;
    use crate::HTTP_REQ_DURATION_HISTOGRAM_BUCKETS;
    use crate::HTTP_REQ_SIZE_HISTOGRAM_BUCKETS;
    use axum::routing::get;
    use axum::Router;
    use axum_test::TestServer;
    use opentelemetry::{global, Context, KeyValue};
    use opentelemetry_sdk::metrics::SdkMeterProvider;
    use prometheus::{Encoder, Registry, TextEncoder};
    use std::sync::Arc;

    async fn metrics_handler() -> String {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
        // return metrics
        String::from_utf8(buffer).unwrap()
    }

    #[tokio::test]
    async fn test_prometheus_exporter() {
        let _cx = Context::current();

        let registry = Registry::new();

        // init prometheus exporter
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()
            .unwrap();

        let provider = SdkMeterProvider::builder().with_reader(exporter).build();

        // init the global meter provider
        global::set_meter_provider(provider.clone());

        let meter = global::meter("my-app");

        // Use two instruments
        let counter = meter.u64_counter("a.counter").with_description("Counts things").build();
        let recorder = meter.u64_histogram("a.histogram").with_description("Records values").build();

        counter.add(100, &[KeyValue::new("key", "value")]);
        recorder.record(100, &[KeyValue::new("key", "value")]);
        provider.force_flush().unwrap();

        // Encode data as text or protobuf
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let mut result = Vec::new();
        encoder.encode(&metric_families, &mut result).expect("encode failed");
        println!("{}", String::from_utf8(result).unwrap());
        provider.shutdown().unwrap();
    }

    #[tokio::test]
    async fn test_builder_with_arced_skipper() {
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(prometheus::default_registry().clone())
            .build()
            .unwrap();
        let provider = SdkMeterProvider::builder().with_reader(exporter).build();

        let metrics = HttpMetricsLayerBuilder::new()
            .with_skipper(crate::PathSkipper::new_with_fn(Arc::new(|s: &str| s.starts_with("/skip"))))
            .with_provider(provider.clone())
            .build();

        let app: Router = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/skip", get(|| async { "skip this handler" }))
            .route("/record", get(|| async { "record this handler" }))
            // add the metrics middleware
            .layer(metrics);

        // Create test server
        let server = TestServer::new(app).unwrap();

        // Make a test request
        let response = server.get("/skip").await;
        assert_eq!(response.status_code(), 200);

        let response = server.get("/record").await;
        assert_eq!(response.status_code(), 200);
        println!(
            "/record response: {:}",
            String::from_utf8(response.as_bytes().to_vec()).unwrap()
        );

        provider.force_flush().unwrap();

        let response = server.get("/metrics").await;
        assert_eq!(response.status_code(), 200);

        let metrics_str = String::from_utf8(response.as_bytes().to_vec()).unwrap();
        println!("/metrics response: {:}", metrics_str);

        assert!(!metrics_str.contains("http_route=\"/skip\""));
        assert!(metrics_str.contains("http_route=\"/record\""));
        provider.shutdown().unwrap();
    }

    #[tokio::test]
    async fn test_custom_buckets() {
        // Custom buckets for testing
        let custom_duration_buckets = vec![0.11, 0.22, 0.33, 0.44];
        let custom_size_buckets = vec![1024.0, 4096.0, 16384.0];

        let registry = Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()
            .unwrap();
        let provider = SdkMeterProvider::builder().with_reader(exporter).build();

        let metrics = HttpMetricsLayerBuilder::new()
            .with_duration_buckets(custom_duration_buckets.clone())
            .with_size_buckets(custom_size_buckets.clone())
            .with_provider(provider.clone())
            .build();

        let app = Router::<()>::new()
            .route("/metrics", get(metrics_handler))
            .route("/test", get(|| async { "test" }))
            .layer(metrics);

        // Create test server
        let server = TestServer::new(app).unwrap();

        // Make a test request
        let response = server.get("/test").await;
        assert_eq!(response.status_code(), 200);

        provider.force_flush().unwrap();

        // Get the metrics output
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let mut output = Vec::new();
        encoder.encode(&metric_families, &mut output).unwrap();
        let metrics_str = String::from_utf8(output).unwrap();

        // print the metrics output
        println!("test_custom_buckets metrics_str: {:}", metrics_str);

        // Verify that our custom buckets are present in the output
        // Duration buckets
        for bucket in custom_duration_buckets {
            assert!(
                metrics_str.contains(&format!("le=\"{}\"", bucket)),
                "Duration bucket {} not found in metrics output",
                bucket
            );
        }

        // Size buckets
        for bucket in custom_size_buckets {
            assert!(
                metrics_str.contains(&format!("le=\"{}\"", bucket)),
                "Size bucket {} not found in metrics output",
                bucket
            );
        }

        provider.shutdown().unwrap();
    }

    #[tokio::test]
    async fn test_default_buckets() {
        let registry = Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .build()
            .unwrap();
        let provider = SdkMeterProvider::builder().with_reader(exporter).build();

        let metrics = HttpMetricsLayerBuilder::new()
            .with_provider(provider.clone())
            .build();

        let app = Router::<()>::new().route("/test", get(|| async { "test" })).layer(metrics);

        // Create test server
        let server = TestServer::new(app).unwrap();

        // Make a test request
        let response = server.get("/test").await;
        assert_eq!(response.status_code(), 200);

        provider.force_flush().unwrap();

        // Get the metrics output
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        let mut output = Vec::new();
        encoder.encode(&metric_families, &mut output).unwrap();
        let metrics_str = String::from_utf8(output).unwrap();
        println!("test_default_buckets metrics_str: {:}", metrics_str);

        // Verify that default buckets are present
        for bucket in HTTP_REQ_DURATION_HISTOGRAM_BUCKETS {
            assert!(
                metrics_str.contains(&format!("le=\"{}\"", bucket)),
                "Default duration bucket {} not found in metrics output",
                bucket
            );
        }

        for bucket in HTTP_REQ_SIZE_HISTOGRAM_BUCKETS {
            assert!(
                metrics_str.contains(&format!("le=\"{}\"", bucket)),
                "Default size bucket {} not found in metrics output",
                bucket
            );
        }

        provider.shutdown().unwrap();
    }
}
