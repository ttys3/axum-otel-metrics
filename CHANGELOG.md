# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

## [0.9.2](https://github.com/ttys3/axum-otel-metrics/compare/v0.9.1...v0.9.2) - 2025-02-13

### Added

- *(metrics)* add custom bucket support for http metrics layer
- add changelog command to Makefile
- add default configuration for git-cliff

### Other

- update CHANGELOG.md
- update changelog template formatting
- update CHANGELOG.md
- update dependencies in Cargo.toml
- derive default for HttpMetricsLayerBuilder
- *(metrics)* remove requests_total counter and related code
- remove provider configuration from builder logic
- *(deps)* update axum requirement from 0.7 to 0.8
- fix typo

### 🚀 Features

- Add default configuration for git-cliff (7fbaefa)
- Add changelog command to Makefile (7849b89)

### 🚜 Refactor

- *(metrics)* Remove requests_total counter and related code (cfad617)
- Derive default for HttpMetricsLayerBuilder (431dfbb)

### 📚 Documentation

- Update changelog template formatting (fb12e69)

### ⚙️ Miscellaneous Tasks

- Fix typo (21aee4e)
- Remove provider configuration from builder logic (5fa718c)
- Update dependencies in Cargo.toml (5a3a263)
- Update CHANGELOG.md (2daf719)

## [0.9.1] - 2024-12-10

### 🚀 Features

- Add script to publish crate to crates-io (19e40e4)

### 📚 Documentation

- Update README with OTLP exporter configuration details (445eb11)
- Update README with prometheus exporter usage link (f754702)
- Update README to improve consistency and formatting (d05183b)

### ⚙️ Miscellaneous Tasks

- Bump version to 0.9.1 (28091ce)

## [0.9.0] - 2024-12-10

### 🐛 Bug Fixes

- Fix compat issue with otel rust 0.26 https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-sdk/CHANGELOG.md#v0260 (5630fb7)

### 🚜 Refactor

- Use with_boundaries API to provide custom bounds for Histogram instruments (6b6fa27)
- *(metrics)* Remove Prometheus exporter and registry references (f01c11f)

### 📚 Documentation

- Fix document (2573371)

### ⚙️ Miscellaneous Tasks

- Update dependencies and refactor SDK meter provider usage (51b1479)
- Bump version to 0.9.0-alpha.3 (a6b6b94)
- Update dependency versions in Cargo.toml (0508e73)
- Update dependencies and improve metrics documentation (0e09dff)
- Update axum-otel-metrics to version 0.9.0 (b7d4110)

## [0.9.0-alpha.2] - 2024-08-03

### 💼 Other

- Update Cargo.lock (2923bb4)
- Update crates (fa3e227)

### ⚙️ Miscellaneous Tasks

- Add Cargo.lock to .gitignore (7948525)
- [**breaking**] Remove Cargo.lock from project tracking (a02bef6)
- Update to opentelemetry-prometheus 0.17 (59bc8ba)
- Bump crate version to v0.9.0-alpha.2 (1f35d7d)

## [0.9.0-alpha.1] - 2024-04-06

### 🚀 Features

- Impl otlp exporter (6c48fb3)

### ⚙️ Miscellaneous Tasks

- Bump version to 0.9.0-alpha.1 (e5b457d)

## [0.8.1] - 2024-04-06

### ⚙️ Miscellaneous Tasks

- Update crates, opentelemetry related crates from 0.21 to 0.22 (880d6a5)
- Update Cargo.lock (1f29898)
- Update example deps (7d38586)
- Update crates (0e83463)
- Bump version to 0.8.1 (9492a8c)

## [0.8.0] - 2023-11-27

### 🚜 Refactor

- Upgrade to axum 0.7 (#88) (4cecbb2)

### 📚 Documentation

- Add Related Projects (15c950f)
- Otel Logs impl is now in Alpha state, and trace is Beta (13740a6)

### ⚙️ Miscellaneous Tasks

- Upgrade to opentelemetry = "0.21" (7ef1caa)
- Update serde to 1.0.193 (1939f93)
- Update examples crates (278dc3e)
- Use dtolnay/rust-toolchain@stable (b6c30f7)

## [0.7.0] - 2023-10-26

### 🚀 Features

- Support http.server.response.size metric (03afca4)

### 🚜 Refactor

- Make metrics name compatible with "Semantic Conventions for HTTP Metrics" (689aa8b)
- Do not repeat the Response type in Service (02a319d)
- *(deps)* Update package versions (03934b9)

### 📚 Documentation

- This is not doc comment (e5b8403)
- Refine doc comments (2a91da5)
- Fix README.md (159d6c1)

### ⚙️ Miscellaneous Tasks

- *(example)* Generate random response (cea8b6b)
- Bump version to 0.6.0 (5694646)
- Add Makefile (f5f3ca1)
- Update crates (f327eb3)
- Bump crate version to 0.7.0 (82a0e4e)

## [0.5.0] - 2023-07-31

### 🐛 Bug Fixes

- Fix tests, refine resource (7056a38)
- Remove Context (007ba7c)

### 💼 Other

- Update crates (9a37835)
- Update crates (f228ba9)
- Upidate to opentelemetry stable version 0.20.0 (6aff312)

### 🚜 Refactor

- Migrate to new impl (the same as go impl v1.16.0) (042687f)

### 📚 Documentation

- Add available labels (a32eaac)

### 🎨 Styling

- Format code (61c0db1)

### ⚙️ Miscellaneous Tasks

- Add unit support (796dc2b)
- Do not use request path if no route matched (0db146d)
- Refine demo, add post data api (9752a30)
- Update examples crates (6125d4f)
- [**breaking**] Bump version (2cce134)

## [0.4.0] - 2023-05-04

### ⚙️ Miscellaneous Tasks

- Update crates (45b3bd7)
- Bump version to 0.4.0 (1a83253)

## [0.3.0] - 2023-03-22

### 🚜 Refactor

- Change counter and histogram metrics name be compatible with golang echo framework's (939ba06)

### ⚙️ Miscellaneous Tasks

- *(examples)* Add nested path metrics demo (b8bacaa)
- Clean up example code (ad287ad)
- Refine example, add user app metrics demo (0ce2cf8)
- *(crate)* Exclude .github (78efe13)
- Update crates (7ad58f1)
- Add TODO about buckets (3ae7b69)
- Bump to 0.3.0 (fb9dafb)

## [0.2.0] - 2022-11-25

### 🚀 Features

- Impl url skipper (c85b3fd)

### 💼 Other

- Update to axum latest main branch (d8546d2)

### 🚜 Refactor

- Make code compatible with axum 0.6.0-rc.5 (965ec7e)

### 🎨 Styling

- Fix tests and doc tests (c3eefa6)

### 🧪 Testing

- Fix tests, format the codes (162db83)

### ⚙️ Miscellaneous Tasks

- Use actions-rs/toolchain@v1 to setup latest stable Rust toolchain, github actions/checkout@v3 update too slow (current msrv is 1.65, but github still got 1.64) (86b085a)
- Migrate to latest axum latest (fef1b12)
- Correct test_builder_with_state_router test (b419fae)
- Use axum 0.6.0 stable version (09b50cb)
- Release 0.2.0 version since axum 0.6.0 is stable (b785e7c)
- Clean up unnecessary pin (f5f93f3)
- Remove unnecessary S bounds on routes method (7f34441)
- Move demo grafana dashboard JSON model to examples dir (c425623)

## [0.1.5] - 2022-10-14

### 🧪 Testing

- Fix cargo test (4a03eb6)

### ⚙️ Miscellaneous Tasks

- Remove unused tokio dep (83fa134)

## [0.1.4] - 2022-10-14

### 📚 Documentation

- Refine crate docs (b9f351a)

## [0.1.3] - 2022-10-14

### 📚 Documentation

- Add code document for HttpMetricsLayerBuilder (65c223d)

### 🎨 Styling

- Cargo clippy and rename examples/prometheus -> examples/axum-metrics-demo (422091f)

### ⚙️ Miscellaneous Tasks

- Tag v0.1.3 (0051d69)

## [0.1.2] - 2022-10-14

### 📚 Documentation

- Using layer() instead of route_layer() (907721f)

### ⚙️ Miscellaneous Tasks

- Tagging v0.1.2 (ff5514e)

## [0.1.1] - 2022-10-14

### 📚 Documentation

- Update README.md (d0200e1)

### 🎨 Styling

- Cargo fmt (50b2aee)

### ⚙️ Miscellaneous Tasks

- Use standalone Cargo.toml for examples (8e1acba)
- Remove unused deps using cargo +nightly udeps (26a752b)
- Remove examples unused deps (b8cf10f)

## [0.1.0] - 2022-10-14

### 🐛 Bug Fixes

- ExporterBuilder will build a PrometheusExporter and set it as global meter_provider. we must fetch the global provider (via global::meter) after it correctly initialized, otherwise it is a noop provider. (cbf3e85)

### 🚜 Refactor

- Refine codes (7cd1e74)
- Move exporter_handler into impl block (ab29c7e)
- Add PromMetricsLayerBuilder (29452c1)

### 📚 Documentation

- Update README.md (43ef588)

### 🎨 Styling

- Format code (8c953b0)
- Format code (0c93aaf)
- Set max_width = 150 (df5d959)
- Cargo clippy (1f0c241)
- Cargo fmt (65ab2a7)

### 🧪 Testing

- Add test_prometheus_exporter (48dee8f)

### ⚙️ Miscellaneous Tasks

- Init commit (bf2bddb)
- Impl layer and service (e71a421)
- Debug prom encode (b126e06)
- Add service_name for prom metrics (011dea7)
- No need use Arc outside, both PrometheusExporter and Metric are interl Arc (89531b0)
- Update .gitignore (a3c00a7)
- Listen on all interfaces. diff path use diff delay (f6c7992)
- Add gen-workload.sh and grafana-dashboard/dashboard.json (7b9337b)
- Update comments (d69fa51)
- Use custom prometheus Registry (63c4c4c)
- Use Context::current() every time record the value (186d174)
- Refine code struct, make this a lib (c98d3c6)
- Prepare for publishing the crate (56b3083)

<!-- generated by git-cliff -->
