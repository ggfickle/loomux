#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script must run on macOS." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

npm install
npm run tauri build

echo "If the macOS build succeeds, look for the DMG under:"
echo "  $ROOT_DIR/src-tauri/target/release/bundle/dmg/"
