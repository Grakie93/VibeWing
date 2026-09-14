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

## Prerequisites

Tauri v2 requires a Rust toolchain in addition to Node.js.

- **Node.js** (>= 18 recommended)
- **Rust** via [rustup](https://rustup.rs/)

### Windows

1. Install Rust with the MSVC toolchain. The easiest way is winget:
   ```powershell
   winget install Rustlang.Rustup
   ```
   Or download and run `rustup-init.exe` from https://rustup.rs. When the prompt asks about **Rust Visual C++ prerequisites**, choose **1) Quick install via the Visual Studio Community installer** — Tauri needs the MSVC linker and Windows APIs.
2. Open a new PowerShell window and verify:
   ```powershell
   rustup default stable-msvc
   cargo --version
   ```
3. If rustup/cargo downloads are slow or timeout (common on first build), use a mirror before running install commands:
   ```powershell
   $env:RUSTUP_DIST_SERVER="https://mirrors.ustc.edu.cn/rust-static"
   $env:RUSTUP_UPDATE_ROOT="https://mirrors.ustc.edu.cn/rust-static/rustup"
   ```
   You can also configure a crates.io mirror in `%USERPROFILE%\.cargo\config.toml` to speed up dependency downloads.

### macOS

A Rust toolchain is required. If your IDE or `~/.cargo/bin` already provides `cargo`, you're set. Otherwise install it from https://rustup.rs.

## Development

```bash
npm install
npm run build
npm run tauri dev
```

Rust source is split into `models`, `storage`, `processes`, and `commands`. Add new capabilities as focused modules instead of growing one application file.
