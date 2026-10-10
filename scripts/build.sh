#!/usr/bin/env sh
set -eu
cargo test
cargo build --release
