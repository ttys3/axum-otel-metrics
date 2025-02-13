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

> by default, it will use the [OTLP Exporter](https://opentelemetry.io/docs/specs/otel/metrics/sdk_exporters/otlp/)
> you can config it via env var:
> `OTEL_EXPORTER_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`

`OTEL_EXPORTER_OTLP_ENDPOINT` default value:

gRPC: `http://localhost:4317`

HTTP: `http://localhost:4318`


`OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` default value:

gRPC: `http://localhost:4317`

HTTP: `http://localhost:4318/v1/metrics`

> for more details, see https://opentelemetry.io/docs/languages/sdk-configuration/otlp-exporter/#endpoint-configuration

```rust
use axum_otel_metrics::HttpMetricsLayerBuilder;
use axum::{response::Html, routing::get, Router};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
 
let exporter = opentelemetry_otlp::MetricExporter::builder()
    .with_http()
    .with_temporality(Temporality::default())
    .build()
    .unwrap();
 
let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
    .with_interval(std::time::Duration::from_secs(30))
    .build()
    .unwrap();

let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
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
```

## Prometheus Exporter

check the doc [Advanced Usage](https://docs.rs/axum-otel-metrics/latest/axum_otel_metrics/#advanced-usage) section to see how to use the prometheus exporter

for prometheus exporter, below metrics will be exported:


`http_server_active_requests` **gauge**

The number of active HTTP requests

`http_server_request_duration_seconds` **histogram**
```
http_server_request_duration_seconds_bucket
http_server_request_duration_seconds_sum
http_server_request_duration_seconds_count
```

`http_server_request_body_size_bytes` **histogram**
```
http_server_request_body_size_bytes_bucket
http_server_request_body_size_bytes_sum
http_server_request_body_size_bytes_count
```

`http_server_response_body_size_bytes` **histogram**
```
http_server_response_body_size_bytes_bucket
http_server_response_body_size_bytes_sum
http_server_response_body_size_bytes_count
```

labels for `http_server_request_duration_seconds`, `http_server_request_body_size_bytes`, `http_server_response_body_size_bytes` :

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
