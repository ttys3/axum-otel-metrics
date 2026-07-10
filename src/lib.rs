//! [axum](https://github.com/tokio-rs/axum) OpenTelemetry Metrics middleware
//!
//! ## Simple Usage: with otlp exporter
//!
//! Meter provider should be configured through [opentelemetry_sdk `global::set_meter_provider`](https://docs.rs/opentelemetry/latest/opentelemetry/global/index.html#global-metrics-api).
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

use axum::http::Response;
use axum::{extract::MatchedPath, http, http::Request};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll::Ready;
use std::task::{ready, Context, Poll};
use std::time::Instant;

use opentelemetry::global;
use opentelemetry::metrics::{Histogram, UpDownCounter};
use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::attribute::{
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, SERVER_ADDRESS, URL_SCHEME,
};
use opentelemetry_semantic_conventions::metric::{
    HTTP_SERVER_ACTIVE_REQUESTS, HTTP_SERVER_REQUEST_BODY_SIZE, HTTP_SERVER_REQUEST_DURATION, HTTP_SERVER_RESPONSE_BODY_SIZE,
};

use tower::{Layer, Service};

use http_body::Body as httpBody; // for `Body::size_hint`
use pin_project_lite::pin_project;

/// OpenTelemetry metric instruments used by the middleware.
#[derive(Clone)]
pub struct Metric {
    pub req_duration: Histogram<f64>,

    pub req_body_size: Histogram<u64>,

    pub res_body_size: Histogram<u64>,

    pub req_active: UpDownCounter<i64>,
}

/// Shared state holding metric instruments, path skipper, and TLS configuration.
#[derive(Clone)]
pub struct MetricState {
    /// The metric instruments.
    pub metric: Metric,

    /// PathSkipper used to skip recording metrics for certain paths.
    skipper: PathSkipper,

    /// Whether the service terminates TLS directly.
    /// Used to determine the `url.scheme` attribute when no forwarded headers are present.
    is_tls: bool,
}

/// Tower [`Service`] wrapper that records OpenTelemetry HTTP server metrics.
#[derive(Clone)]
pub struct HttpMetrics<S> {
    pub(crate) state: Arc<MetricState>,
    service: S,
}

/// Tower [`Layer`] that wraps services with [`HttpMetrics`] for recording HTTP server metrics.
#[derive(Clone)]
pub struct HttpMetricsLayer {
    pub(crate) state: Arc<MetricState>,
}

// as https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md#metric-httpserverrequestduration spec
// This metric SHOULD be specified with ExplicitBucketBoundaries of [ 0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1, 2.5, 5, 7.5, 10 ].
// the unit of the buckets is second
const HTTP_REQ_DURATION_HISTOGRAM_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
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
    /// starts with `/favicon.ico``.
    ///
    /// This is the default implementation used when
    /// building an HttpMetricsLayerBuilder from scratch.
    fn default() -> Self {
        Self::new(|s| s.starts_with("/favicon.ico"))
    }
}

/// Builder for constructing an [`HttpMetricsLayer`] with custom configuration.
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

    /// Set a custom [`PathSkipper`] to skip recording metrics for certain paths.
    pub fn with_skipper(mut self, skipper: PathSkipper) -> Self {
        self.skipper = skipper;
        self
    }

    /// Set custom histogram bucket boundaries for request duration (in seconds).
    pub fn with_duration_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.duration_buckets = Some(buckets);
        self
    }

    /// Set custom histogram bucket boundaries for request/response body size (in bytes).
    pub fn with_size_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.size_buckets = Some(buckets);
        self
    }

    /// Set whether the server terminates TLS directly (without a reverse proxy).
    /// When `true`, `url.scheme` is always `"https"`, skipping forwarded header detection.
    pub fn with_tls(mut self, is_tls: bool) -> Self {
        self.is_tls = is_tls;
        self
    }

    /// Set a custom [`MeterProvider`](opentelemetry::metrics::MeterProvider). Defaults to the global provider.
    pub fn with_provider<P>(mut self, provider: P) -> Self
    where
        P: opentelemetry::metrics::MeterProvider + Send + Sync + 'static,
    {
        self.provider = Some(Arc::new(provider));
        self
    }

    /// Build the [`HttpMetricsLayer`] with the configured settings.
    pub fn build(self) -> HttpMetricsLayer {
        let provider = self.provider.unwrap_or_else(|| global::meter_provider());

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
            .with_description("Duration of HTTP server requests.")
            .with_boundaries(duration_buckets)
            .build();

        let req_size = meter
            .u64_histogram(HTTP_SERVER_REQUEST_BODY_SIZE)
            .with_unit("By")
            .with_description("Size of HTTP server request bodies.")
            .with_boundaries(size_buckets.clone())
            .build();

        let res_size = meter
            .u64_histogram(HTTP_SERVER_RESPONSE_BODY_SIZE)
            .with_unit("By")
            .with_description("Size of HTTP server response bodies.")
            .with_boundaries(size_buckets)
            .build();

        // no u64_up_down_counter because up_down_counter maybe < 0 since it allow negative values
        let req_active = meter
            .i64_up_down_counter(HTTP_SERVER_ACTIVE_REQUESTS)
            .with_unit("{request}")
            .with_description("Number of active HTTP server requests.")
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

        HttpMetricsLayer {
            state: Arc::new(meter_state),
        }
    }
}

/// Map the request method to a low-cardinality `&'static str` label.
///
/// Per the OTel spec, methods not in the well-known set MUST be reported as `_OTHER`
/// so clients cannot inflate the `http.request.method` label cardinality.
#[inline]
fn method_label(method: &http::Method) -> &'static str {
    match *method {
        http::Method::GET => "GET",
        http::Method::POST => "POST",
        http::Method::PUT => "PUT",
        http::Method::DELETE => "DELETE",
        http::Method::HEAD => "HEAD",
        http::Method::OPTIONS => "OPTIONS",
        http::Method::CONNECT => "CONNECT",
        http::Method::PATCH => "PATCH",
        http::Method::TRACE => "TRACE",
        _ => "_OTHER",
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
        state: Arc<MetricState>,
        path: Arc<str>,
        method: &'static str,
        url_scheme: &'static str,
        host: Arc<str>,
        req_body_size: u64,
        completed: bool,
    }

    impl<F> PinnedDrop for ResponseFuture<F> {
        fn drop(this: Pin<&mut Self>) {
            let this = this.project();
            // The inner future was dropped before completing (client disconnect,
            // timeout layer, graceful shutdown, panic unwinding). The regular
            // decrement in `poll` never ran, so do it here to keep
            // `http.server.active_requests` from drifting upwards forever.
            if !*this.completed {
                this.state.metric.req_active.add(
                    -1,
                    &[
                        KeyValue::new(HTTP_REQUEST_METHOD, *this.method),
                        KeyValue::new(URL_SCHEME, *this.url_scheme),
                    ],
                );
            }
        }
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
        // for scheme, see github.com/labstack/echo/v4@v4.11.1/context.go
        // we can not use req.uri().scheme() since for non-absolute uri, it is always None
        let url_scheme: &'static str = if self.state.is_tls {
            "https"
        } else {
            let forwarded = req
                .headers()
                .get("X-Forwarded-Proto")
                .and_then(|v| v.to_str().ok())
                .or_else(|| req.headers().get("X-Forwarded-Protocol").and_then(|v| v.to_str().ok()));
            match forwarded {
                Some(s) if s.eq_ignore_ascii_case("https") => "https",
                Some(_) => "http",
                None => {
                    if req.headers().get("X-Forwarded-Ssl").and_then(|v| v.to_str().ok()) == Some("on") {
                        "https"
                    } else {
                        match req.headers().get("X-Url-Scheme").and_then(|v| v.to_str().ok()) {
                            Some(s) if s.eq_ignore_ascii_case("https") => "https",
                            Some(_) => "http",
                            None => "http",
                        }
                    }
                }
            }
        };

        let method = method_label(req.method());

        let start = Instant::now();
        let path: Arc<str> = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
            Arc::from(matched_path.as_str())
        } else {
            Arc::from("")
        };

        let host: Arc<str> = Arc::from(
            req.headers()
                .get(http::header::HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("unknown"),
        );

        let req_body_size = compute_request_body_size(&req);

        let inner = self.service.call(req);

        // ref https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md#metric-httpserveractive_requests
        // http.request.method and url.scheme is required
        //
        // Incremented only after the inner `call` succeeded, so a panic there cannot
        // leak a `+1`; every other exit path is balanced by `poll` or `PinnedDrop`.
        self.state.metric.req_active.add(
            1,
            &[
                KeyValue::new(HTTP_REQUEST_METHOD, method),
                KeyValue::new(URL_SCHEME, url_scheme),
            ],
        );

        ResponseFuture {
            inner,
            start,
            method,
            path,
            host,
            req_body_size: req_body_size as u64,
            state: self.state.clone(),
            url_scheme,
            completed: false,
        }
    }
}

fn compute_request_body_size<T>(req: &Request<T>) -> usize {
    req.headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}

impl<F, B: httpBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.inner.poll(cx));

        // Always decrement active requests, regardless of success or error
        this.state.metric.req_active.add(
            -1,
            &[
                KeyValue::new(HTTP_REQUEST_METHOD, *this.method),
                KeyValue::new(URL_SCHEME, *this.url_scheme),
            ],
        );
        // From here on the cancellation guard in `PinnedDrop` must stay silent.
        *this.completed = true;

        let response = match result {
            Ok(response) => response,
            Err(err) => return Poll::Ready(Err(err)),
        };

        if (this.state.skipper.skip)(this.path.as_ref()) {
            return Poll::Ready(Ok(response));
        }

        let latency = this.start.elapsed().as_secs_f64();
        let status = response.status().as_u16() as i64;

        let res_body_size = response.body().size_hint().upper().unwrap_or(0);

        let labels = [
            KeyValue::new(HTTP_REQUEST_METHOD, *this.method),
            KeyValue::new(HTTP_ROUTE, this.path.clone()),
            KeyValue::new(HTTP_RESPONSE_STATUS_CODE, status),
            // server.address: Name of the local HTTP server that received the request.
            // Determined by using the first of the following that applies
            //
            // 1. The primary server name of the matched virtual host. MUST only include host identifier.
            // 2. Host identifier of the request target if it's sent in absolute-form.
            // 3. Host identifier of the Host header
            KeyValue::new(SERVER_ADDRESS, this.host.clone()),
            KeyValue::new(URL_SCHEME, *this.url_scheme),
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
    use axum::routing::get;
    use axum::Router;
    use axum_test::TestServer;
    use opentelemetry::{global, Context, KeyValue};
    use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
    use std::sync::Arc;

    fn create_test_provider() -> SdkMeterProvider {
        let exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_temporality(Temporality::default())
            .build()
            .unwrap();

        let reader = PeriodicReader::builder(exporter)
            .with_interval(std::time::Duration::from_secs(30))
            .build();

        SdkMeterProvider::builder().with_reader(reader).build()
    }

    #[tokio::test]
    async fn test_otlp_exporter() {
        let _cx = Context::current();

        let provider = create_test_provider();

        // init the global meter provider
        global::set_meter_provider(provider.clone());

        let meter = global::meter("my-app");

        // Use two instruments
        let counter = meter.u64_counter("a.counter").with_description("Counts things").build();
        let recorder = meter.u64_histogram("a.histogram").with_description("Records values").build();

        counter.add(100, &[KeyValue::new("key", "value")]);
        recorder.record(100, &[KeyValue::new("key", "value")]);

        // In test environment, OTLP exporter may fail to flush without real endpoint
        let _ = provider.force_flush();
        let _ = provider.shutdown();
    }

    #[tokio::test]
    async fn test_builder_with_arced_skipper() {
        let provider = create_test_provider();

        let metrics = HttpMetricsLayerBuilder::new()
            .with_skipper(crate::PathSkipper::new_with_fn(Arc::new(|s: &str| s.starts_with("/skip"))))
            .with_provider(provider.clone())
            .build();

        let app: Router = Router::new()
            .route("/skip", get(|| async { "skip this handler" }))
            .route("/record", get(|| async { "record this handler" }))
            // add the metrics middleware
            .layer(metrics);

        // Create test server
        let server = TestServer::new(app);

        // Make a test request
        let response = server.get("/skip").await;
        assert_eq!(response.status_code(), 200);

        let response = server.get("/record").await;
        assert_eq!(response.status_code(), 200);
        println!(
            "/record response: {:}",
            String::from_utf8(response.as_bytes().to_vec()).unwrap()
        );

        // In test environment, OTLP exporter may fail to flush without real endpoint
        let _ = provider.force_flush();
        let _ = provider.shutdown();
    }

    #[tokio::test]
    async fn test_custom_buckets() {
        // Custom buckets for testing
        let custom_duration_buckets = vec![0.11, 0.22, 0.33, 0.44];
        let custom_size_buckets = vec![1024.0, 4096.0, 16384.0];

        let provider = create_test_provider();

        let metrics = HttpMetricsLayerBuilder::new()
            .with_duration_buckets(custom_duration_buckets.clone())
            .with_size_buckets(custom_size_buckets.clone())
            .with_provider(provider.clone())
            .build();

        let app = Router::<()>::new().route("/test", get(|| async { "test" })).layer(metrics);

        // Create test server
        let server = TestServer::new(app);

        // Make a test request
        let response = server.get("/test").await;
        assert_eq!(response.status_code(), 200);

        // In test environment, OTLP exporter may fail to flush without real endpoint
        let _ = provider.force_flush();
        let _ = provider.shutdown();
    }

    #[tokio::test]
    async fn test_default_buckets() {
        let provider = create_test_provider();

        let metrics = HttpMetricsLayerBuilder::new().with_provider(provider.clone()).build();

        let app = Router::<()>::new().route("/test", get(|| async { "test" })).layer(metrics);

        // Create test server
        let server = TestServer::new(app);

        // Make a test request
        let response = server.get("/test").await;
        assert_eq!(response.status_code(), 200);

        // In test environment, OTLP exporter may fail to flush without real endpoint
        let _ = provider.force_flush();
        let _ = provider.shutdown();
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let provider = create_test_provider();

        let metrics = HttpMetricsLayerBuilder::new().with_provider(provider.clone()).build();

        let app = Router::<()>::new()
            .route("/test", get(|| async { "test response" }))
            .layer(metrics);

        // Create test server
        let server = TestServer::new(app);

        // Make multiple test requests to generate metrics
        for _ in 0..5 {
            let response = server.get("/test").await;
            assert_eq!(response.status_code(), 200);
        }

        // In test environment, OTLP exporter may fail to flush without real endpoint
        let _ = provider.force_flush();
        let _ = provider.shutdown();
    }

    fn create_in_memory_provider() -> (opentelemetry_sdk::metrics::InMemoryMetricExporter, SdkMeterProvider) {
        let exporter = opentelemetry_sdk::metrics::InMemoryMetricExporter::default();
        let reader = PeriodicReader::builder(exporter.clone()).build();
        let provider = SdkMeterProvider::builder().with_reader(reader).build();
        (exporter, provider)
    }

    #[tokio::test]
    async fn test_active_requests_not_leaked_on_cancel() {
        use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};

        let (exporter, provider) = create_in_memory_provider();

        let layer = HttpMetricsLayerBuilder::new().with_provider(provider.clone()).build();

        let mut svc = tower::Layer::layer(
            &layer,
            tower::service_fn(|_req: axum::http::Request<axum::body::Body>| async {
                std::future::pending::<Result<axum::http::Response<axum::body::Body>, std::convert::Infallible>>().await
            }),
        );

        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/pending")
            .body(axum::body::Body::empty())
            .unwrap();

        // Drop the in-flight future before it completes, simulating a client
        // disconnect or a timeout layer cancelling the request.
        let fut = tower::Service::call(&mut svc, req);
        drop(fut);

        provider.force_flush().unwrap();

        let metrics = exporter.get_finished_metrics().unwrap();
        let mut found = false;
        for rm in &metrics {
            for sm in rm.scope_metrics() {
                for m in sm.metrics() {
                    if m.name() == "http.server.active_requests" {
                        if let AggregatedMetrics::I64(MetricData::Sum(sum)) = m.data() {
                            for dp in sum.data_points() {
                                found = true;
                                assert_eq!(
                                    dp.value(),
                                    0,
                                    "active_requests must return to 0 when the request future is dropped"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert!(found, "http.server.active_requests data points not found");
    }

    #[tokio::test]
    async fn test_histogram_labels_follow_semconv() {
        use opentelemetry::Value;
        use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};

        let (exporter, provider) = create_in_memory_provider();

        let metrics = HttpMetricsLayerBuilder::new().with_provider(provider.clone()).build();

        let app = Router::<()>::new().route("/test", get(|| async { "ok" })).layer(metrics);

        let server = TestServer::new(app);
        let response = server.get("/test").await;
        assert_eq!(response.status_code(), 200);

        provider.force_flush().unwrap();

        let exported = exporter.get_finished_metrics().unwrap();
        let mut found = false;
        for rm in &exported {
            for sm in rm.scope_metrics() {
                for m in sm.metrics() {
                    if m.name() == "http.server.request.duration" {
                        if let AggregatedMetrics::F64(MetricData::Histogram(hist)) = m.data() {
                            for dp in hist.data_points() {
                                found = true;
                                let attrs: Vec<_> = dp.attributes().collect();
                                let get = |key: &str| attrs.iter().find(|kv| kv.key.as_str() == key).map(|kv| kv.value.clone());
                                assert_eq!(get("http.request.method"), Some(Value::from("GET")));
                                assert_eq!(get("http.route"), Some(Value::from("/test")));
                                assert_eq!(get("url.scheme"), Some(Value::from("http")));
                                assert_eq!(get("http.response.status_code"), Some(Value::I64(200)));
                                assert!(get("server.address").is_some());
                            }
                        }
                    }
                }
            }
        }
        assert!(found, "http.server.request.duration data points not found");
    }
}
