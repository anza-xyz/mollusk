#!/bin/bash

set -e

agave-install init 2.1.0
rm -rf target
cargo build
./scripts/build-test-programs.sh
cargo +nightly-2024-11-22 fmt --all -- --check
cargo +nightly-2024-11-22 clippy --all --all-features -- -D warnings
cargo test --all-features
