# VibeWing 2.0 architecture preview

This directory contains the Tauri 2 migration for VibeWing. It is developed by Grakie93 and is not yet the default release build.

## Architecture

- Vue 3 + TypeScript for the interface.
- Tauri 2 + Rust for project processes, logs, Git, settings, credentials, and AI requests.
- Existing VibeWing JSON data is migrated on first launch.

The migration is incremental. Keep using the root Electron build until this preview reaches feature parity on macOS and Windows.

## Development

```bash
npm install
npm run build
npx tauri dev
```

Rust source is split into `models`, `storage`, `processes`, and `commands`. Add new capabilities as focused modules instead of growing one application file.
