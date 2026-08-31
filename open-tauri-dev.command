#!/bin/sh
# Open the single packaged Tauri development build.
# Keep this separate from start.command, which launches the Electron release.
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
APP_PATH="$ROOT_DIR/releases/VibeWing-Tauri-Dev.app"

if [ ! -d "$APP_PATH" ]; then
  echo "VibeWing-Tauri-Dev.app was not found at:"
  echo "  $APP_PATH"
  echo
  echo "Build it from v2 with:"
  echo "  npm run tauri build -- --bundles app"
  read -r _
  exit 1
fi

open "$APP_PATH"
