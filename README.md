# axum-otel-metrics

[![Build status](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/axum-otel-metrics)](https://crates.io/crates/axum-otel-metrics)
[![Documentation](https://docs.rs/axum-otel-metrics/badge.svg)](https://docs.rs/axum-otel-metrics)

axum OpenTelemetry metrics middleware with prometheus exporter

[axum](https://github.com/tokio-rs/axum) is an ergonomic and modular web framework built with Tokio, Tower, and Hyper

be default, the metrics will be exported at `/metrics` endpoint.
and below metrics will be exported:

request_duration_seconds **histogram**
```
request_duration_seconds_bucket
request_duration_seconds_sum
request_duration_seconds_count
```

requests_total **counter**

```
requests_total
```

## Usage

```rust
use axum_otel_metrics::HttpMetricsLayerBuilder;

let metrics = HttpMetricsLayerBuilder::new()
    .build();

let app = Router::new()
    // export metrics at `/metrics` endpoint
    .merge(metrics.routes())
    .route("/", get(handler))
    .route("/hello", get(handler))
    .route("/world", get(handler))
    // add the metrics middleware
    .layer(metrics);
```

## Usage with `State`

```rust
use axum_otel_metrics::HttpMetricsLayerBuilder;

#[derive(Clone)]
pub struct SharedState {
}

let state = SharedState {
};

let metrics = HttpMetricsLayerBuilder::new()
    .build();

let app = Router::new()
    // export metrics at `/metrics` endpoint
    .merge(metrics.routes::<SharedState>())
    .route("/", get(handler))
    .route("/hello", get(handler))
    .route("/world", get(handler))
    // add the metrics middleware
    .layer(metrics)
    .with_state(state.clone());
```

## OpenTelemetry Rust Instrumentation Status and Releases

https://opentelemetry.io/docs/instrumentation/rust/#status-and-releases

| Traces                                                                                           | Metrics | Logs                |
|--------------------------------------------------------------------------------------------------|---------|---------------------|
| [Stable](https://opentelemetry.io/docs/reference/specification/versioning-and-stability/#stable) | Alpha   | Not yet implemented |

## OpenTelemetry Metrics Exporter

**Push Metric Exporter** https://opentelemetry.io/docs/reference/specification/metrics/sdk/#push-metric-exporter

**Pull Metric Exporter** https://opentelemetry.io/docs/reference/specification/metrics/sdk/#pull-metric-exporter


### exporters

https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/

In-memory https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/in-memory/

Prometheus https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/prometheus/

OTLP https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/otlp/

Standard output https://opentelemetry.io/docs/reference/specification/metrics/sdk_exporters/stdout/

## Metrics Data Model

https://opentelemetry.io/docs/reference/specification/metrics/data-model/
