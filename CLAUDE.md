# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WeChat Cleaner (微剪) is a Tauri 2.x desktop app that removes redundant files from WeChat's PC storage. It performs cross-directory deduplication (SHA-256 hash matching against user-specified archive dirs) and version convergence (detecting `filename(n).ext` patterns, keeping the newest). The app is written in Vue 3 + TypeScript (frontend) and Rust (backend), communicating via Tauri IPC.

## Commands

| Task | Command |
|---|---|
| Full Tauri dev (frontend + backend) | `npm run tauri dev` |
| Frontend dev server only | `npm run dev` (port 1420) |
| Type check + production build | `npm run build` |
| Run frontend unit tests | `npm test` |
| Watch mode tests | `npm run test:watch` |
| Standalone type check | `npx vue-tsc --noEmit` |
| Build distributable installer | `npm run tauri build` |
| Rust type check | `cargo check` (in `src-tauri/`) |

## Architecture

**IPC boundary:** Frontend calls Rust via `invoke('command_name', { params })` from `@tauri-apps/api`. There are 18 Tauri commands defined in `src-tauri/src/commands.rs`, registered in `src-tauri/src/lib.rs`.

**Frontend (`src/`):** 3-page wizard flow (Config → Scan → Results) with Vue Router. A single Pinia store (`stores/app.ts`) holds all shared state. `ScanView` polls `get_scan_progress` at 500ms intervals. `ResultView` uses infinite scroll with paged loading (100 groups per page).

**Backend (`src-tauri/src/`):** Organized into modules:
- `scanner/` — 3-phase pipeline: walk → hash → dedup. `walker.rs` traverses directories (skipping `.db-shm`, `.db-wal`, `.ini`). `hash.rs` does full SHA-256 + head/tail partial hash (first+last 4KB) for fast pre-filtering. `dedup.rs` implements cross-directory dedup and version convergence algorithms.
- `commands.rs` — All IPC commands. Uses `AppState` with `Arc<Mutex<...>>` for shared state and `Arc<AtomicBool>` for cancel/pause signals.
- `config/wechat.rs` — Config persistence as JSON, WeChat account auto-detection.
- `cleaner/trash.rs` — File deletion (trash or permanent) with WeChat-running check.
- `platform/` — `PlatformOps` trait with Windows/macOS implementations.
- `error.rs` — `AppError` with `thiserror`, serialized over IPC.
- `debug.rs` — File-based debug logging (`debug.log` next to executable).

## Key Technical Details

- Scan runs in `std::thread::spawn` (not tokio) because Tauri manages its own async runtime.
- Hashing uses Rayon for parallel SHA-256 computation.
- Frontend uses `<script setup>` Composition API exclusively.
- Config auto-saves with 500ms debounce.
- The `@tanstack/vue-virtual` dependency is present but not currently used in views.

## Language

The UI and README are in Chinese (Simplified). Code identifiers and comments are in English.
