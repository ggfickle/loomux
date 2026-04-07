# Loomux

A lightweight Tauri desktop app for viewing and managing local tmux sessions on macOS, now with a menu bar dropdown for quick attach.

## What Loomux does

- Lists local tmux sessions
- Creates, renames, and deletes tmux sessions
- Opens a selected session in macOS Terminal.app and attaches directly
- Adds a macOS menu bar / tray dropdown that shows sessions without opening the full window
- Keeps the main window available for fuller management and debugging

## Menu bar mode

Loomux now creates a tray / menu bar item.

From the menu bar you can:

- open the main Loomux window
- refresh the tmux session list
- see detected tmux sessions directly in the dropdown
- click a session to launch Terminal.app and run `tmux attach -t <session>`
- quit Loomux

Notes:

- The main window still works normally.
- Closing the main window hides it instead of fully quitting, so the menu bar dropdown stays available.
- Double-clicking the tray icon reopens the main window.

## macOS tmux detection improvements

The main bug reported by the user was: tmux sessions really existed in Terminal, but Loomux showed an empty list after installing / launching from Finder.

### Likely root cause

On macOS, apps launched from Finder often do **not** inherit the same shell environment as Terminal.

That especially affects tmux in two ways:

1. **PATH mismatch**
   - Finder may not see Homebrew's tmux path.
   - Loomux already handled this with common binary-path probing, and now also tries `command -v tmux` from the login shell.

2. **Socket / `TMUX_TMPDIR` mismatch**
   - Existing tmux sessions may live on a socket under a custom `TMUX_TMPDIR` or a non-default socket name.
   - A Finder-launched app may miss that env var, so plain `tmux list-sessions` talks to the wrong socket and returns empty / no-server.
   - In that case the app can even create a new tmux server while still not seeing the user's real sessions.

### What Loomux now does

Loomux keeps the binary path detection and adds more robust session probing:

- checks `TMUX_BINARY_PATH`
- checks `tmux` on current `PATH`
- checks the login shell's `command -v tmux`
- checks common macOS tmux install paths:
  - `/opt/homebrew/bin/tmux`
  - `/usr/local/bin/tmux`
  - `/usr/bin/tmux`
  - `/bin/tmux`
  - `/opt/local/bin/tmux`
- reads `TMUX_TMPDIR` from:
  - the app process environment
  - the login shell environment
- scans likely tmux socket directories such as:
  - `${TMUX_TMPDIR}/tmux-<uid>/`
  - `/tmp/tmux-<uid>/`
  - `/private/tmp/tmux-<uid>/`
- tries structured session listing first:
  - `tmux list-sessions -F "#{session_name}\t#{session_attached}\t#{session_windows}"`
- falls back to:
  - `tmux ls`
- records the chosen socket path and detection notes, then surfaces them in the UI

### UI debugging help

The main window now shows:

- resolved tmux binary path
- chosen session probe / socket source
- debug notes when detection had to scan alternate sockets or login-shell env

That makes Finder-vs-Terminal mismatches much easier to diagnose.

## Project layout

- `src/` — React UI
- `src-tauri/` — Rust backend and Tauri app config
- `scripts/build-macos-dmg.sh` — macOS-only helper for building the DMG

## Run on Linux

This Linux host can validate most logic, but not the real macOS Terminal / menu bar behavior.

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

On Linux:

- tmux session discovery works
- create / rename / delete flows work
- the tray plumbing builds
- **Open in Terminal** still returns an unsupported-platform error because Terminal.app automation is macOS-only

## Build the frontend

```bash
npm run build
```

## Build on macOS

A real macOS machine is required for final validation.

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

## Install and first open on macOS

If you download a DMG or `.app` built outside the App Store and without Apple notarization, macOS may block it with a message like:

- "Loomux is damaged and can’t be opened"
- or similar Gatekeeper warnings

This usually does **not** mean the app binary is actually corrupted. It usually means macOS attached a quarantine flag to the downloaded app.

### Recommended install steps

1. Download the latest Loomux DMG from GitHub Releases.
2. Open the DMG.
3. Drag `Loomux.app` into `/Applications`.
4. In Terminal, remove the quarantine flag:

```bash
xattr -dr com.apple.quarantine /Applications/Loomux.app
```

5. Then open Loomux from `/Applications`.

### Optional checks

See whether macOS attached quarantine metadata:

```bash
xattr -l /Applications/Loomux.app
```

If you see `com.apple.quarantine`, remove it with the command above.

### Notes

- Right-click → **Open** may also help in some cases, but the `xattr -dr ...` command is the most direct fix.
- A future signed + notarized release should remove this manual step, but for now users should expect to do it once after install.

## Validation checklist

### Verified on this Linux host

- `npm run build`
- `cargo check`
- `cargo test`
- tray feature compiles with Tauri `tray-icon`
- structured tmux parser and `tmux ls` fallback parser unit tests pass

### Important note about Rust on this host

The system `cargo` here is too old for the current locked dependency set.
Using the rustup-managed stable toolchain works:

```bash
PATH="$HOME/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH" cargo check
PATH="$HOME/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH" cargo test
```

### Still requires macOS validation

- Finder-launched `.app` sees existing real tmux sessions
- menu bar dropdown updates correctly against the user's tmux setup
- clicking a session opens Terminal.app and attaches to the correct socket/session
- closing the main window leaves the menu bar mode alive as expected
- `.app` / `.dmg` bundling and first-run automation permissions

## Risks / limitations

- Loomux currently picks the strongest detected tmux target (typically the socket with the real sessions) rather than merging every possible tmux server into one UI.
- If a user intentionally runs multiple independent tmux socket trees, Loomux prefers the most relevant detected one.
- Terminal automation on macOS still depends on AppleScript / permission prompts.
- Final UX needs a real macOS smoke test after packaging.

## Rollout

1. Validate on Linux for build/test sanity.
2. Move to a macOS machine.
3. Launch from Finder and confirm the existing tmux sessions appear.
4. Confirm the menu bar dropdown shows sessions and attaches correctly.
5. Build the `.app` / `.dmg` and smoke test the installed app.
