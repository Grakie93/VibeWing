# VibeWing 2.0 architecture preview

This directory contains the Tauri 2 migration for VibeWing. It is developed by Grakie93 and is not yet the default release build.

## Architecture

- Vue 3 + TypeScript for the interface.
- Tauri 2 + Rust for project processes, logs, Git, settings, credentials, and AI requests.
- Existing VibeWing JSON data is migrated on first launch.

The migration is incremental. Keep using the root Electron build until this preview reaches feature parity on macOS and Windows.

## Which build should I open?

- Electron release: double-click the root `start.command`.
- Tauri packaged development build: double-click the root `open-tauri-dev.command`. It opens the only supported packaged Tauri app at `releases/VibeWing-Tauri-Dev.app`.
- Tauri source development window: double-click `v2/start-tauri-dev.command` (or run the command below). This is for development and may show a Vite/Rust console.

Do not open `.app` files copied from `v2/src-tauri/target`; those are build intermediates and can be stale. The `releases` directory is local-only and is intentionally ignored by Git.

## Development

```bash
npm install
npm run build
npm run tauri dev
```

Rust source is split into `models`, `storage`, `processes`, and `commands`. Add new capabilities as focused modules instead of growing one application file.
