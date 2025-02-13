# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### 🚀 Features

- Add default configuration for git-cliff
- Add changelog command to Makefile

### 🚜 Refactor

- *(metrics)* Remove requests_total counter and related code
- Derive default for HttpMetricsLayerBuilder

### ⚙️ Miscellaneous Tasks

- Fix typo
- Remove provider configuration from builder logic
- Update dependencies in Cargo.toml

## [0.9.1] - 2024-12-10

### 🚀 Features

- Add script to publish crate to crates-io

### 📚 Documentation

- Update README with OTLP exporter configuration details
- Update README with prometheus exporter usage link
- Update README to improve consistency and formatting

### ⚙️ Miscellaneous Tasks

- Bump version to 0.9.1

## [0.9.0] - 2024-12-10

### 🐛 Bug Fixes

- Fix compat issue with otel rust 0.26 https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-sdk/CHANGELOG.md#v0260

### 🚜 Refactor

- Use with_boundaries API to provide custom bounds for Histogram instruments
- *(metrics)* Remove Prometheus exporter and registry references

### 📚 Documentation

- Fix document

### ⚙️ Miscellaneous Tasks

- Update dependencies and refactor SDK meter provider usage
- Bump version to 0.9.0-alpha.3
- Update dependency versions in Cargo.toml
- Update dependencies and improve metrics documentation
- Update axum-otel-metrics to version 0.9.0

## [0.9.0-alpha.2] - 2024-08-03

### 💼 Other

- Update Cargo.lock
- Update crates

### ⚙️ Miscellaneous Tasks

- Add Cargo.lock to .gitignore
- [**breaking**] Remove Cargo.lock from project tracking
- Update to opentelemetry-prometheus 0.17
- Bump crate version to v0.9.0-alpha.2

## [0.9.0-alpha.1] - 2024-04-06

### 🚀 Features

- Impl otlp exporter

### ⚙️ Miscellaneous Tasks

- Bump version to 0.9.0-alpha.1

## [0.8.1] - 2024-04-06

### ⚙️ Miscellaneous Tasks

- Update crates, opentelemetry related crates from 0.21 to 0.22
- Update Cargo.lock
- Update example deps
- Update crates
- Bump version to 0.8.1

## [0.8.0] - 2023-11-27

### 🚜 Refactor

- Upgrade to axum 0.7 (#88)

### 📚 Documentation

- Add Related Projects
- Otel Logs impl is now in Alpha state, and trace is Beta

### ⚙️ Miscellaneous Tasks

- Upgrade to opentelemetry = "0.21"
- Update serde to 1.0.193
- Update examples crates
- Use dtolnay/rust-toolchain@stable

## [0.7.0] - 2023-10-26

### 🚀 Features

- Support http.server.response.size metric

### 🚜 Refactor

- Make metrics name compatible with "Semantic Conventions for HTTP Metrics"
- Do not repeat the Response type in Service
- *(deps)* Update package versions

### 📚 Documentation

- This is not doc comment
- Refine doc comments
- Fix README.md

### ⚙️ Miscellaneous Tasks

- *(example)* Generate random response
- Bump version to 0.6.0
- Add Makefile
- Update crates
- Bump crate version to 0.7.0

## [0.5.0] - 2023-07-31

### 🐛 Bug Fixes

- Fix tests, refine resource
- Remove Context

### 💼 Other

- Update crates
- Update crates
- Upidate to opentelemetry stable version 0.20.0

### 🚜 Refactor

- Migrate to new impl (the same as go impl v1.16.0)

### 📚 Documentation

- Add available labels

### 🎨 Styling

- Format code

### ⚙️ Miscellaneous Tasks

- Add unit support
- Do not use request path if no route matched
- Refine demo, add post data api
- Update examples crates
- [**breaking**] Bump version

## [0.4.0] - 2023-05-04

### ⚙️ Miscellaneous Tasks

- Update crates
- Bump version to 0.4.0

## [0.3.0] - 2023-03-22

### 🚜 Refactor

- Change counter and histogram metrics name be compatible with golang echo framework's

### ⚙️ Miscellaneous Tasks

- *(examples)* Add nested path metrics demo
- Clean up example code
- Refine example, add user app metrics demo
- *(crate)* Exclude .github
- Update crates
- Add TODO about buckets
- Bump to 0.3.0

## [0.2.0] - 2022-11-25

### 🚀 Features

- Impl url skipper

### 💼 Other

- Update to axum latest main branch

### 🚜 Refactor

- Make code compatible with axum 0.6.0-rc.5

### 🎨 Styling

- Fix tests and doc tests

### 🧪 Testing

- Fix tests, format the codes

### ⚙️ Miscellaneous Tasks

- Use actions-rs/toolchain@v1 to setup latest stable Rust toolchain, github actions/checkout@v3 update too slow (current msrv is 1.65, but github still got 1.64)
- Migrate to latest axum latest
- Correct test_builder_with_state_router test
- Use axum 0.6.0 stable version
- Release 0.2.0 version since axum 0.6.0 is stable
- Clean up unnecessary pin
- Remove unnecessary S bounds on routes method
- Move demo grafana dashboard JSON model to examples dir

## [0.1.5] - 2022-10-14

### 🧪 Testing

- Fix cargo test

### ⚙️ Miscellaneous Tasks

- Remove unused tokio dep

## [0.1.4] - 2022-10-14

### 📚 Documentation

- Refine crate docs

## [0.1.3] - 2022-10-14

### 📚 Documentation

- Add code document for HttpMetricsLayerBuilder

### 🎨 Styling

- Cargo clippy and rename examples/prometheus -> examples/axum-metrics-demo

### ⚙️ Miscellaneous Tasks

- Tag v0.1.3

## [0.1.2] - 2022-10-14

### 📚 Documentation

- Using layer() instead of route_layer()

### ⚙️ Miscellaneous Tasks

- Tagging v0.1.2

## [0.1.1] - 2022-10-14

### 📚 Documentation

- Update README.md

### 🎨 Styling

- Cargo fmt

### ⚙️ Miscellaneous Tasks

- Use standalone Cargo.toml for examples
- Remove unused deps using cargo +nightly udeps
- Remove examples unused deps

## [0.1.0] - 2022-10-14

### 🐛 Bug Fixes

- ExporterBuilder will build a PrometheusExporter and set it as global meter_provider. we must fetch the global provider (via global::meter) after it correctly initialized, otherwise it is a noop provider.

### 🚜 Refactor

- Refine codes
- Move exporter_handler into impl block
- Add PromMetricsLayerBuilder

### 📚 Documentation

- Update README.md

### 🎨 Styling

- Format code
- Format code
- Set max_width = 150
- Cargo clippy
- Cargo fmt

### 🧪 Testing

- Add test_prometheus_exporter

### ⚙️ Miscellaneous Tasks

- Init commit
- Impl layer and service
- Debug prom encode
- Add service_name for prom metrics
- No need use Arc outside, both PrometheusExporter and Metric are interl Arc
- Update .gitignore
- Listen on all interfaces. diff path use diff delay
- Add gen-workload.sh and grafana-dashboard/dashboard.json
- Update comments
- Use custom prometheus Registry
- Use Context::current() every time record the value
- Refine code struct, make this a lib
- Prepare for publishing the crate

<!-- generated by git-cliff -->
