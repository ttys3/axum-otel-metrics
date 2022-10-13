# axum-otel-metrics

OpenTelemetry Metrics middleware for [axum](https://github.com/tokio-rs/axum) http server

axum is an ergonomic and modular web framework built with Tokio, Tower, and Hyper

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
