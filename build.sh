#!/usr/bin/env sh
set -eu
ra3_root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec cargo run --release --manifest-path "$ra3_root/Cargo.toml" -- build --root "$ra3_root" "$@"
