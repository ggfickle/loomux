# Loomux

A lightweight Tauri desktop app for viewing and managing local tmux sessions, with macOS Terminal.app opening support.

## Chosen stack

- **Tauri v2** for the desktop shell and macOS packaging path
- **React + TypeScript + Vite** for a small maintainable UI
- **Rust** for local process execution (`tmux`, `pgrep`, `ps`, `osascript`)

Why this stack:
- lighter than Electron
- realistic macOS app packaging story
- simple local command integration
- practical to scaffold on Linux while leaving macOS-only packaging for a Mac

## Features in v1

- Lists current tmux sessions
- Shows tmux session count
- Shows tmux process count
- Shows session names, attached state, and window count
- Creates tmux sessions
- Renames tmux sessions
- Deletes tmux sessions with confirmation
- Refreshes the session list after create, rename, and delete actions
- Provides an **Open in Terminal** action wired for macOS
- Configures Tauri bundle targets for `.app` and `.dmg`

## Project layout

- `src/` — React UI
- `src-tauri/` — Rust backend and Tauri app config
- `scripts/build-macos-dmg.sh` — macOS-only helper for building the DMG

## Run on Linux

This Linux host can scaffold and validate most of the app, but not the real macOS Terminal integration or DMG output.

### Prerequisites

- Node.js 18+
- npm
- Rust toolchain
- Tauri Linux system dependencies if you want to run the desktop shell locally
- `tmux` installed locally if you want live session data

### Install dependencies

```bash
npm install
```

### Run the frontend only

```bash
npm run dev
```

### Run the Tauri app locally on Linux

```bash
npm run tauri dev
```

On Linux, tmux session listing and create/rename/delete flows work. The **Open in Terminal** action intentionally returns an unsupported-platform error because Terminal.app automation is macOS-only.

### tmux binary detection

The app now tries to find `tmux` in this order:

1. `TMUX_BINARY_PATH` environment variable
2. `tmux` from `PATH`
3. Common macOS paths:
   - `/opt/homebrew/bin/tmux`
   - `/usr/local/bin/tmux`
   - `/usr/bin/tmux`
   - `/bin/tmux`
   - `/opt/local/bin/tmux`

This matters on macOS because GUI apps launched from Finder often do not inherit the same `PATH` as Terminal. The detected path is shown in the UI.

### Build the frontend

```bash
npm run build
```

## Build on macOS

A real macOS machine is required for these steps.

### Prerequisites

- Xcode Command Line Tools
- Node.js 18+
- npm
- Rust toolchain
- Tauri prerequisites for macOS
- `tmux`

### Local development on macOS

```bash
npm install
npm run tauri dev
```

### Build the `.app` and `.dmg`

```bash
npm run tauri build
```

Or use the helper:

```bash
./scripts/build-macos-dmg.sh
```

Expected output location:

```text
src-tauri/target/release/bundle/dmg/
```

## Linux limitations

These parts cannot be fully finished or validated on this Linux host:

1. **Terminal.app automation**
   - The app uses `osascript` on macOS to open Terminal.app and run `tmux attach -t <session>`.
   - That path is compiled behind a macOS target gate and cannot be exercised on Linux.

2. **DMG creation**
   - Tauri macOS bundling depends on Apple tooling and a macOS runtime.
   - The config is in place, but the actual `.dmg` build must run on macOS.

3. **macOS permission prompts**
   - Terminal/automation permissions may prompt the user on first use.
   - That needs a manual macOS verification pass.

## Validation checklist

### What works on Linux now

- Project structure and app scaffold
- tmux session discovery logic
- tmux create, rename, and delete logic
- frontend UI build path
- platform-gated open-session command behavior

### What still requires macOS

- verifying Terminal.app launches and attaches correctly
- building the signed or unsigned macOS `.app`
- generating and testing the `.dmg`

## Risks

- tmux output can vary slightly across versions; the parser currently targets the standard fields used in `tmux list-sessions -F`.
- If `tmux` is missing or no tmux server is running, listing returns an empty session list.
- Create/rename/delete actions surface tmux's stderr directly, so duplicate names or missing sessions show the underlying tmux error.
- On macOS, first-run automation permissions may affect the open-session flow.

## Rollout

1. Validate the app on Linux for session discovery and UI behavior.
2. Move to a macOS machine.
3. Run `npm run tauri dev` and test Terminal.app attach behavior.
4. Run `npm run tauri build` to produce the `.app` and `.dmg`.
5. Smoke-test the DMG-installed app against a real local tmux setup.

## Rollback

This is a new standalone project, so rollback is simple: revert or remove this repo directory if the scaffold is not wanted.
