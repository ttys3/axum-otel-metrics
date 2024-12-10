# axum-otel-metrics

[![Build status](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/ttys3/axum-otel-metrics/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/axum-otel-metrics)](https://crates.io/crates/axum-otel-metrics)
[![Documentation](https://docs.rs/axum-otel-metrics/badge.svg)](https://docs.rs/axum-otel-metrics)

axum OpenTelemetry metrics middleware

supported exporters:

- [otlp](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/otlp/)
- [prometheus](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/prometheus/)

follow [Semantic Conventions for HTTP Metrics](https://github.com/open-telemetry/semantic-conventions/blob/main/docs/http/http-metrics.md)

[axum](https://github.com/tokio-rs/axum) is an ergonomic and modular web framework built with Tokio, Tower, and Hyper

## Usage

```rust
use axum_otel_metrics::HttpMetricsLayerBuilder;

let metrics = HttpMetricsLayerBuilder::new()
    .build();

let app = Router::new()
    .route("/", get(handler))
    .route("/hello", get(handler))
    .route("/world", get(handler))
    // add the metrics middleware
    .layer(metrics);
```

## prometheus exporter

for prometheus exporter, below metrics will be exported:


`requests_total` **counter**

```
requests_total
```

`http_server_active_requests` **gauge**

The number of active HTTP requests

`http_server_request_duration_seconds` **histogram**
```
http_server_request_duration_seconds_bucket
http_server_request_duration_seconds_sum
http_server_request_duration_seconds_count
```

`http_server_request_size_bytes` **histogram**
```
http_server_request_size_bytes_bucket
http_server_request_size_bytes_sum
http_server_request_size_bytes_count
```

`http_server_response_size_bytes_` **histogram**
```
http_server_response_size_bytes_bucket
http_server_response_size_bytes_sum
http_server_response_size_bytes_count
```

labels for `requests_total`,
`http_server_request_duration_seconds`, `http_server_request_size_bytes`,
`http_server_response_size_bytes` :

```
http_request_method
http_route
http_response_status_code
server_address
```

labels for `http_server_active_requests` :

```
http_request_method
url_scheme
```

## OpenTelemetry Rust Instrumentation Status and Releases

https://opentelemetry.io/docs/instrumentation/rust/#status-and-releases

| Traces                                                                                           | Metrics | Logs                |
|--------------------------------------------------------------------------------------------------|---------|---------------------|
| [Beta](https://github.com/open-telemetry/oteps/blob/main/text/0232-maturity-of-otel.md#beta) | Beta   | Beta |

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


## Related Projects

https://github.com/nlopes/actix-web-prom
> Actix-web middleware to expose Prometheus metrics

https://github.com/sd2k/rocket_prometheus
> Prometheus fairing and handler for Rocket

https://github.com/Ptrskay3/axum-prometheus
> axum-prometheus relies on metrics.rs and its ecosystem to collect and export metrics - for instance for Prometheus, metrics_exporter_prometheus is used as a backend to interact with Prometheus.
