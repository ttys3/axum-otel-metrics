# OpenTelemetry Metrics Best Practices for `axum-otel-metrics`

This document outlines key best practices for configuring and using OpenTelemetry metrics in your Axum applications with the `axum-otel-metrics` middleware. 

Following these guidelines ensures high-performance metrics collection, keeps memory usage low, prevents cardinality explosion, and guarantees that metrics are not lost during application restarts.

---

## 1. Exporter Selection: Migrate to OTLP

* **Best Practice:** Always use the OpenTelemetry Protocol (OTLP) push exporter rather than a pull-based Prometheus exporter inside the application process.
* **Why:** 
  * The `opentelemetry-prometheus` crate has been deprecated and suffers from unmaintained dependencies (such as the legacy `protobuf` crate with known vulnerabilities).
  * Prometheus natively supports OTLP ingestion. Using OTLP decouples metrics collection from your application process, offloading serialization and scrape management to a local OpenTelemetry Collector or Prometheus agent.

---

## 2. Preventing Cardinality Explosion (Avoiding Memory Leaks)

* **Best Practice:** Keep the cardinality of metric attributes (labels) low. Never use raw user IDs, UUIDs, or dynamic request paths as metric attributes.
* **Why:** Each unique combination of attribute values creates a new time series in memory. Emitting infinite unique values (like `/users/12345/profile` as `http.route`) causes the Metrics SDK memory usage to grow indefinitely, leading to out-of-memory (OOM) crashes.
* **How `axum-otel-metrics` handles this:**
  * By default, the middleware maps the request path to `http.route` using Axum's `MatchedPath` (e.g. `/users/:id/profile`). This groups all dynamic paths into a single static route pattern, keeping metric cardinality low and stable.
  * The HTTP request method is normalized to a static set (e.g. `GET`, `POST`). Unrecognized or custom HTTP methods are automatically recorded as `_OTHER` to prevent malicious or accidental cardinality inflation.

---

## 3. Filtering Out Unwanted Paths via `PathSkipper`

* **Best Practice:** Skip recording metrics for high-volume, low-value routes such as health checks (`/healthz`), readiness probes, metrics scrape endpoints, or static assets.
* **Why:** Collecting metrics for health probes increases CPU overhead, allocates memory for unused time series, and pollutes your dashboard visualizations with noise.
* **How to configure:**
  Use the `PathSkipper` inside the `HttpMetricsLayerBuilder`:

```rust
use axum_otel_metrics::{HttpMetricsLayerBuilder, PathSkipper};

let metrics = HttpMetricsLayerBuilder::new()
    .with_skipper(PathSkipper::new(|path| {
        path.starts_with("/health") || path.starts_with("/metrics")
    }))
    .build();
```

---

## 4. Ensure Graceful Shutdown and Metrics Flushing

* **Best Practice:** Always call `provider.shutdown()` when your Axum server terminates to guarantee all buffered metrics are exported.
* **Why:** The OpenTelemetry Metrics SDK utilizes a `PeriodicReader` which buffers and exports metrics on an interval (e.g., every 30 seconds). If the application exits suddenly (due to a deploy or restart) without calling `shutdown()`, the final batch of metrics in memory will be lost.
* **How to implement:**
  Hook into your async runtime shutdown signal and flush the provider:

```rust
use opentelemetry::global;
use opentelemetry_sdk::metrics::SdkMeterProvider;

#[tokio::main]
async fn main() {
    let provider = SdkMeterProvider::builder()
        // ... configure reader and resource ...
        .build();
    global::set_meter_provider(provider.clone());

    let app = axum::Router::new()
        .layer(HttpMetricsLayerBuilder::new().build());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    // Serve the app with a graceful shutdown signal
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    // CRITICAL: Force flush and shutdown the provider before exiting
    if let Err(err) = provider.shutdown() {
        eprintln!("Error shutting down MeterProvider: {:?}", err);
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install ctrl+c signal handler");
}
```

---

## 5. Optimizing Attribute Allocations on Hot Paths

* **Best Practice:** When recording custom metrics within your handlers:
  * Pass attribute arrays as slices (e.g. `&[KeyValue::new(...)]`) instead of allocating vectors (`vec![...]`).
  * Ensure consistent ordering of attributes (ideally sorted lexicographically by key).
* **Why:** 
  * Passing slice references avoids heap allocations for every single metric observation.
  * The OpenTelemetry SDK performs a lookup to match the incoming attributes to an internal accumulator. Providing attributes in a consistent, sorted order allows the SDK to resolve the lookup much faster and minimizes memory footprint.

```rust
// GOOD: Slice representation, sorted attributes
counter.add(1, &[
    KeyValue::new("action", "login"),
    KeyValue::new("status", "success")
]);

// AVOID: Allocates vector on the heap and dynamic ordering
// counter.add(1, &vec![KeyValue::new("status", "success"), KeyValue::new("action", "login")]);
```

---

## 6. Tuning Histogram Buckets

* **Best Practice:** Customize the histogram duration and size buckets to match your application's expected payload sizes and response latency profiles.
* **Why:** The default bucket boundaries (ranging from 5ms to 10s for durations) are generic. Having too many buckets or buckets that do not cover your target distribution wastes storage and reduces resolution where it matters.
* **How to configure:**

```rust
let metrics = HttpMetricsLayerBuilder::new()
    // Configure duration buckets in seconds (e.g., 1ms to 2.5s)
    .with_duration_buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.5])
    // Configure body size buckets in bytes (e.g., 100B to 1MB)
    .with_size_buckets(vec![100.0, 500.0, 1000.0, 10_000.0, 100_000.0, 1_000_000.0])
    .build();
```

---

## 7. Global Resource Configuration

* **Best Practice:** Bind common application metadata (e.g., `service.name`, `service.version`, `deployment.environment`) to the `Resource` during the setup of `SdkMeterProvider`.
* **Why:** Specifying service details on the global Resource ensures that every metric series exported by the application includes this context automatically. This avoids the need to manually attach these common fields to every single request-level metric or handler-level counter.

```rust
use opentelemetry::{global, KeyValue};
use opentelemetry_semantic_conventions::attribute::{
    DEPLOYMENT_ENVIRONMENT_NAME, SERVICE_NAME, SERVICE_VERSION
};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;

let resource = Resource::builder()
    .with_attributes(vec![
        KeyValue::new(SERVICE_NAME, "my-axum-service"),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, "production"),
    ])
    .build();

let provider = SdkMeterProvider::builder()
    .with_resource(resource)
    // ... configure readers ...
    .build();

// CRITICAL: Set the global meter provider so the middleware and other
// parts of the application can resolve it automatically.
global::set_meter_provider(provider.clone());
```

---

## 8. Reference Specifications & Official Resources

To dive deeper into OpenTelemetry guidelines and keep your observability strategy aligned with standard conventions, refer to the following official resources:

### General & Metrics Specifications
* **[OpenTelemetry Specification - Metrics API & SDK](https://opentelemetry.io/docs/specs/otel/metrics/)**: Core guidelines on metrics data models, aggregation, temporality, and exporter configurations.
* **[OpenTelemetry Rust Metrics Guidelines](https://github.com/open-telemetry/opentelemetry-rust/blob/main/docs/metrics.md)**: Official documentation detailing the state of metrics in the Rust SDK, API usage, memory allocation details, and concurrency considerations.

### Semantic Conventions (SemConv)
* **[OpenTelemetry Semantic Conventions Guide](https://opentelemetry.io/docs/specs/semconv/)**: The central repository for all standardized attribute names and structural rules for tracing, metrics, and logs.
* **[HTTP Server Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/http/http-server/)**: Specific guidelines for HTTP server metric names (e.g., `http.server.request.duration`) and required attributes (e.g., `http.request.method`, `http.route`, `url.scheme`, `http.response.status_code`, `server.address`).
* **[HTTP Client Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/http/http-client/)**: Guidelines for instrumenting outgoing HTTP requests (e.g., `http.client.request.duration`), useful if your Axum service calls external APIs.

### Generative AI Observability (GenAI)
As Large Language Models (LLMs) and Generative AI applications grow, OpenTelemetry has standardized conventions to measure cost, safety, performance, and model interactions:
* **[Generative AI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/)**: Standardized attributes for tracing and metrics concerning generative AI model calls, covering input parameters (e.g., `gen_ai.request.model`, `gen_ai.request.temperature`) and output metadata (e.g., token usage counts like `gen_ai.usage.input_tokens` and `gen_ai.usage.output_tokens`).
* **[OpenTelemetry for Generative AI Blog Post](https://opentelemetry.io/blog/2024/otel-generative-ai/)**: An official overview of GenAI observability highlighting the integration of **Traces** (tracking lifecycle/parameters), **Metrics** (cost/latency/request volume), and **Events** (capturing prompt/response contents via Logs API with `OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT` for compliance and auditing).

