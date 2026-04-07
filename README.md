# Loomux

A lightweight Tauri desktop app for viewing and managing local tmux sessions on macOS, with a menu bar dropdown and a configurable terminal launcher.

## What Loomux does

- Lists local tmux sessions
- Creates, renames, and deletes tmux sessions
- Opens a selected session in your preferred terminal on macOS
- Adds a macOS menu bar / tray dropdown that shows sessions without opening the full window
- Keeps the main window available for fuller management and tmux detection debugging

## Terminal preference

Loomux now has a simple in-app **Terminal preference** section.

Supported options:

- **Auto**
- **Terminal**
- **iTerm**
- **Ghostty**
- **Tabby**
- **Custom command**

### Current behavior

- **Auto** currently prefers installed terminals in this order:
  1. Ghostty
  2. iTerm
  3. Tabby
  4. Terminal
- **Terminal** uses the existing AppleScript Terminal.app flow.
- **iTerm** uses iTerm's AppleScript support.
- **Ghostty** launches Ghostty's app executable and runs `tmux attach` through `-e /bin/zsh -lc ...`.
- **Tabby** launches Tabby's app executable and uses its `run` CLI mode.
- **Custom command** runs your own shell command template.

The saved preference is used by:

- the main window **Open session** button
- the menu bar / tray session items

### Custom command placeholders

When using **Custom command**, these placeholders are supported:

- `{{command}}` → full `tmux attach` command
- `{{tmux_binary}}` → quoted tmux binary path
- `{{socket_arg}}` → quoted `-S ...` argument when a socket is known
- `{{session_name}}` → quoted session name

Example:

```bash
open -na Ghostty --args -e /bin/zsh -lc "{{command}}"
```

If no placeholder is present, Loomux appends the full attach command to the end of your custom command.

## Menu bar mode

Loomux creates a tray / menu bar item.

From the menu bar you can:

- open the main Loomux window
- refresh the tmux session list
- see detected tmux sessions directly in the dropdown
- click a session to launch your saved terminal preference and attach to it
- quit Loomux

Notes:

- The main window still works normally.
- Closing the main window hides it instead of fully quitting, so the menu bar dropdown stays available.
- Double-clicking the tray icon reopens the main window.

## macOS tmux detection improvements

The main bug reported earlier was: tmux sessions really existed in Terminal, but Loomux showed an empty list after installing / launching from Finder.

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

### What Loomux does now

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

The main window shows:

- resolved tmux binary path
- chosen session probe / socket source
- debug notes when detection had to scan alternate sockets or login-shell env

## macOS icon packaging status

To reduce the odds of the installed app showing a blank icon, Loomux now bundles a fuller Tauri/macOS icon set instead of relying on one PNG alone.

### What changed

- Added generated multi-size icon assets under `src-tauri/icons/`
- Added a bundled macOS `.icns` file
- Updated `src-tauri/tauri.conf.json` to explicitly point Tauri at:
  - `icons/32x32.png`
  - `icons/128x128.png`
  - `icons/128x128@2x.png`
  - `icons/icon.icns`
  - `icons/icon.ico`
- Added a reproducible icon generation command:

```bash
npm run icons
```

which runs:

```bash
tauri icon src-tauri/icons/icon-source.png -o src-tauri/icons
```

### Important limitation

This is still a **best-effort packaging fix from Linux**.

It should make the bundle structure more correct for macOS, but blank-icon behavior can still depend on:

- Finder icon cache state
- how the `.app` was built and copied into `/Applications`
- Gatekeeper / quarantine behavior
- the quality and size of the original source art

The current source art is still based on a 512×512 PNG, so a higher-resolution or vector source would be a worthwhile future improvement.

## Project layout

- `src/` — React UI
- `src-tauri/` — Rust backend and Tauri app config
- `scripts/build-macos-dmg.sh` — macOS-only helper for building the DMG

## Run on Linux

This Linux host can validate most logic, but not the real macOS terminal-launching or Finder icon behavior.

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
- terminal preference UI loads and saves
- tray plumbing builds
- **Open session** still returns an unsupported-platform error because macOS terminal launching is the only supported desktop-launch path right now

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

### Rebuild icon assets

```bash
npm run icons
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

## Validation checklist

### Verified on this Linux host

- `npm run build`
- `cargo check`
- `cargo test`
- tray feature compiles with Tauri `tray-icon`
- tmux parser unit tests pass
- terminal-preference persistence and Rust command wiring compile on Linux

### Important note about Rust on this host

The system `cargo` here is too old for the current locked dependency set.
Using the rustup-managed stable toolchain works:

```bash
PATH="$HOME/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH" cargo check
PATH="$HOME/.rustup/toolchains/stable-aarch64-unknown-linux-gnu/bin:$PATH" cargo test
```

### Still requires macOS validation

- Finder-launched `.app` sees existing real tmux sessions
- Auto terminal choice behaves as expected on a real Mac
- Terminal / iTerm / Ghostty / Tabby launch flows work against real installed apps
- custom command templates behave as expected with real user commands
- menu bar dropdown uses the saved terminal preference correctly
- `.app` / `.dmg` icon appearance after install into `/Applications`
- whether Finder still shows a blank icon in any install/cache edge cases
- first-run automation / permission prompts

## Risks / limitations

- Loomux currently picks the strongest detected tmux target (typically the socket with the real sessions) rather than merging every possible tmux server into one UI.
- If a user intentionally runs multiple independent tmux socket trees, Loomux prefers the most relevant detected one.
- Ghostty and Tabby launch paths are implemented from their app CLI entry points, but still need real macOS smoke testing.
- Custom commands are intentionally simple string templates, not a full scripting UI.
- Final icon and terminal UX still needs a real macOS packaging pass.

## Rollout

1. Validate on Linux for build/test sanity.
2. Move to a macOS machine.
3. Launch from Finder and confirm the existing tmux sessions appear.
4. Confirm the terminal preference setting works for each desired terminal.
5. Confirm the menu bar dropdown opens sessions using that same preference.
6. Build the `.app` / `.dmg` and smoke test the installed app, including icon appearance.
