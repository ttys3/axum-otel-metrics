#!/usr/bin/env bash

set -eou pipefail

# need GIT_TOKEN
env GIT_CLIFF_CONFIG=./cliff.toml RUST_LOG=info,release_plz=debug release-plz -v release-pr


