#!/usr/bin/env bash

set -eou pipefail

cargo build && ./target/debug/axum-metrics-demo


