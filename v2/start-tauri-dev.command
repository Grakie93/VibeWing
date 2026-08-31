#!/bin/sh
# Start the Tauri development window from the v2 source tree.
# Use open-tauri-dev.command when you want the packaged app instead.
set -eu

V2_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
PROJECT_ROOT="$(CDPATH= cd -- "$V2_DIR/.." && pwd)"
TOOLING_DIR="$PROJECT_ROOT/.tooling"

cd "$V2_DIR"
export RUSTUP_HOME="$TOOLING_DIR/rustup"
export CARGO_HOME="$TOOLING_DIR/cargo"
export PATH="$CARGO_HOME/bin:$PATH"

exec npm run tauri dev
