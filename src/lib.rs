//! [axum](https://github.com/tokio-rs/axum) OpenTelemetry Metrics middleware with prometheus exporter
//!
//! ## Simple Usage
//! ```
//! use axum_otel_metrics::HttpMetricsLayerBuilder;
//! use axum::{response::Html, routing::get, Router};
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
//! ## Advanced Usage
//! ```
//! use axum_otel_metrics::HttpMetricsLayerBuilder;
//! use axum::{response::Html, routing::get, Router};
//!
//! let metrics = HttpMetricsLayerBuilder::new()
//! .with_service_name(env!("CARGO_PKG_NAME").to_string())
//! .with_service_version(env!("CARGO_PKG_VERSION").to_string())
//! .with_prefix("axum_metrics_demo".to_string())
//! .with_labels(vec![("env".to_string(), "testing".to_string())].into_iter().collect())
//! .build();
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
use std::sync::Arc;
use std::time::Instant;

use std::future::Future;
use std::pin::Pin;
use std::task::Poll::{self, Ready};
use std::task::{Context};

use opentelemetry::{global, metrics::{Counter, Histogram, UpDownCounter}, KeyValue};

use tower::{Layer, Service};

use futures_util::ready;
use http_body::Body as HttpBody;
use pin_project_lite::pin_project;

/// The metrics we use in the middleware.
#[derive(Clone)]
pub struct Metric {
    pub requests_total: Counter<u64>,
    pub req_duration: Histogram<f64>,
    pub req_size: Histogram<u64>,
    pub res_size: Histogram<u64>,
    pub req_active: UpDownCounter<i64>,
}

#[derive(Clone)]
pub struct MetricState {
    /// Holds the metrics we use in the middleware.
    pub metric: Metric,

    /// PathSkipper used to skip some paths for not recording metrics.
    skipper: PathSkipper,

    /// Whether the service is running as a TLS server or not.
    /// This is used to help determine the `url.scheme` OpenTelemetry meter attribute.
    is_tls: bool,

    /// Additional labels to include with each metric.
    labels: Vec<KeyValue>,
}

/// The service wrapper.
#[derive(Clone)]
pub struct HttpMetrics<S> {
    pub(crate) state: MetricState,

    /// Inner service which is wrapped by this middleware.
    service: S,
}

#[derive(Clone)]
pub struct HttpMetricsLayer {
    /// The metric state, used by the middleware handler.
    pub(crate) state: MetricState,
}

/// A helper that instructs the metrics layer to ignore certain paths.
#[derive(Clone)]
pub struct PathSkipper {
    skip: Arc<dyn Fn(&str) -> bool + 'static + Send + Sync>,
}

impl PathSkipper {
    /// Returns a `PathSkipper` that skips recording metrics
    /// for requests whose path, when passed to `fn`, returns `true`.
    pub fn new(skip: fn(&str) -> bool) -> Self {
        Self {
            skip: Arc::new(skip),
        }
    }

    /// Dynamic variant of `PathSkipper::new`.
    pub fn new_with_fn(skip: Arc<dyn Fn(&str) -> bool + 'static + Send + Sync>) -> Self {
        Self { skip }
    }
}

impl Default for PathSkipper {
    /// Returns a `PathSkipper` that skips any path which starts with `/metrics` or `/favicon.ico`.
    fn default() -> Self {
        Self::new(|s| s.starts_with("/metrics") || s.starts_with("/favicon.ico"))
    }
}

#[derive(Clone)]
pub struct HttpMetricsLayerBuilder {
    prefix: Option<String>,
    labels: Vec<KeyValue>,
    skipper: PathSkipper,
    is_tls: bool,
}

impl Default for HttpMetricsLayerBuilder {
    fn default() -> Self {
        Self {
            prefix: None,
            labels: Vec::new(),
            skipper: PathSkipper::default(),
            is_tls: false,
        }
    }
}

impl HttpMetricsLayerBuilder {
    pub fn new() -> Self {
        HttpMetricsLayerBuilder::default()
    }


    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn with_labels(mut self, labels: std::collections::HashMap<String, String>) -> Self {
        self.labels = labels
            .into_iter()
            .map(|(k, v)| KeyValue::new(k, v))
            .collect();
        self
    }
    pub fn with_skipper(mut self, skipper: PathSkipper) -> Self {
        self.skipper = skipper;
        self
    }

    pub fn with_tls(mut self, is_tls: bool) -> Self {
        self.is_tls = is_tls;
        self
    }

    pub fn build(self) -> HttpMetricsLayer {
        let meter = global::meter("axum-app");

        // Initialize metrics.
        let requests_total = meter
            .u64_counter("requests")
            .with_description("How many HTTP requests processed, partitioned by status code and HTTP method.")
            .init();

        let req_duration = meter
            .f64_histogram("http.server.request.duration")
            .with_unit("s")
            .with_description("The HTTP request latencies in seconds.")
            .init();

        let req_size = meter
            .u64_histogram("http.server.request.size")
            .with_unit("By")
            .with_description("The HTTP request sizes in bytes.")
            .init();

        let res_size = meter
            .u64_histogram("http.server.response.size")
            .with_unit("By")
            .with_description("The HTTP response sizes in bytes.")
            .init();

        let req_active = meter
            .i64_up_down_counter("http.server.active_requests")
            .with_description("The number of active HTTP requests.")
            .init();

        let metric_state = MetricState {
            metric: Metric {
                requests_total,
                req_duration,
                req_size,
                res_size,
                req_active,
            },
            labels: self.labels,
            skipper: self.skipper,
            is_tls: self.is_tls,
        };

        HttpMetricsLayer {
            state: metric_state,
        }
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
        req_size: u64,
    }
}

impl<S, R, ResBody> Service<Request<R>> for HttpMetrics<S>
where
    S: Service<Request<R>, Response = Response<ResBody>>,
    ResBody: HttpBody,
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
                if req.headers().get("X-Forwarded-Ssl").map(|v| v.to_str().unwrap()) == Some("on") {
                    return "https".to_string();
                }
                if let Some(scheme) = req.headers().get("X-Url-Scheme") {
                    return scheme.to_str().unwrap().to_string();
                } else {
                    return "http".to_string();
                }
            })()
        };

        // Record active requests.
        let mut active_labels = vec![
            KeyValue::new("http.request.method", req.method().as_str().to_string()),
            KeyValue::new("url.scheme", url_scheme.clone()),
        ];
        active_labels.extend_from_slice(&self.state.labels);
        self.state.metric.req_active.add(
            1,
            &active_labels,
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

        let req_size = compute_approximate_request_size(&req);

        ResponseFuture {
            inner: self.service.call(req),
            start,
            method,
            path,
            host,
            req_size: req_size as u64,
            state: self.state.clone(),
            url_scheme,
        }
    }
}

/// Compute approximate request size.
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

impl<F, B: HttpBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<B>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let response = ready!(this.inner.poll(cx))?;

        // Decrease active requests.
        let mut active_labels = vec![
            KeyValue::new("http.request.method", this.method.clone()),
            KeyValue::new("url.scheme", this.url_scheme.clone()),
        ];
        active_labels.extend_from_slice(&this.state.labels);
        this.state.metric.req_active.add(
            -1,
            &active_labels,
        );

        if (this.state.skipper.skip)(this.path.as_str()) {
            return Poll::Ready(Ok(response));
        }

        let latency = this.start.elapsed().as_secs_f64();
        let status = response.status().as_u16().to_string();

        let res_size = response.body().size_hint().upper().unwrap_or(0);

        let mut labels = vec![
            KeyValue::new("http.request.method", this.method.clone()),
            KeyValue::new("http.route", this.path.clone()),
            KeyValue::new("http.response.status_code", status),
            KeyValue::new("server.address", this.host.clone()),
        ];
        labels.extend_from_slice(&this.state.labels);

        this.state.metric.requests_total.add(1, &labels);
        this.state.metric.req_size.record(*this.req_size, &labels);
        this.state.metric.res_size.record(res_size, &labels);
        this.state.metric.req_duration.record(latency, &labels);

        Ready(Ok(response))
    }
}
