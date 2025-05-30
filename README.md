# axum-otel-metrics

[![Build status](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/axum-otel-metrics)](https://crates.io/crates/axum-otel-metrics)
[![Documentation](https://docs.rs/axum-otel-metrics/badge.svg)](https://docs.rs/axum-otel-metrics)

axum OpenTelemetry metrics middleware with OTLP exporter

follows [Semantic Conventions for HTTP Metrics](https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md)

[axum](https://github.com/tokio-rs/axum) is an ergonomic and modular web framework built with Tokio, Tower, and Hyper

## Usage

Uses the [OTLP Exporter](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/otlp/) to send metrics to OpenTelemetry collector.

You can configure it via environment variables:
- `OTEL_EXPORTER_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`

`OTEL_EXPORTER_OTLP_ENDPOINT` default value:
- gRPC: `http://localhost:4317`
- HTTP: `http://localhost:4318`

`OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` default value:
- gRPC: `http://localhost:4317`
- HTTP: `http://localhost:4318/v1/metrics`

> For more details, see https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/#endpoint-configuration

```rust
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum::{response::Html, routing::get, Router};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
use opentelemetry::global;
 
let exporter = opentelemetry_otlp::MetricExporter::builder()
    .with_http()
    .with_temporality(Temporality::default())
    .build()
    .unwrap();
 
let reader = PeriodicReader::builder(exporter)
    .with_interval(std::time::Duration::from_secs(30))
    .build();

let provider = SdkMeterProvider::builder()
    .with_reader(reader)
    .build();

// TODO: ensure defer run `provider.shutdown()?;`

global::set_meter_provider(provider.clone());

let metrics = HttpMetricsLayerBuilder::new()
    .build();

let app = Router::new()
    .route("/", get(handler))
    .route("/hello", get(handler))
    .route("/world", get(handler))
    // add the metrics middleware
    .layer(metrics);

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

## Exported Metrics

The following metrics are exported following OpenTelemetry semantic conventions:

**http.server.active_requests** (UpDownCounter)
- The number of active HTTP requests
- Labels: `http.request.method`, `url.scheme`

**http.server.request.duration** (Histogram) 
- The HTTP request latencies in seconds
- Labels: `http.request.method`, `http.route`, `http.response.status_code`, `server.address`

**http.server.request.body.size** (Histogram)
- The HTTP request sizes in bytes  
- Labels: `http.request.method`, `http.route`, `http.response.status_code`, `server.address`

**http.server.response.body.size** (Histogram)
- The HTTP response sizes in bytes
- Labels: `http.request.method`, `http.route`, `http.response.status_code`, `server.address`

## OpenTelemetry Rust Instrumentation Status and Releases

https://opentelemetry.io/docs/instrumentation/rust/#status-and-releases

| Traces | Metrics | Logs |
|--------|---------|------|
| [Beta](https://github.com/open-telemetry/oteps/blob/main/text/0232-maturity-of-otel.md#beta) | Beta | Beta |

## OpenTelemetry Metrics Exporter

**Push Metric Exporter** https://opentelemetry.io/docs/reference/specification/metrics/sdk/#push-metric-exporter

**Pull Metric Exporter** https://opentelemetry.io/docs/reference/specification/metrics/sdk/#pull-metric-exporter

### exporters

https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/

- In-memory https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/in-memory/
- OTLP https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/otlp/
- Standard output https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/stdout/

## Metrics Data Model

https://opentelemetry.io/docs/reference/specification/metrics/data-model/

## Related Projects

- https://github.com/nlopes/actix-web-prom - Actix-web middleware to expose Prometheus metrics
- https://github.com/sd2k/rocket_prometheus - Prometheus fairing and handler for Rocket
- https://github.com/Ptrskay3/axum-prometheus - axum-prometheus relies on metrics.rs and its ecosystem to collect and export metrics
