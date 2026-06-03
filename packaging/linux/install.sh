#!/usr/bin/env bash
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

install -d "$PREFIX/bin"
install -m 0755 "$ROOT_DIR/target/release/pin" "$PREFIX/bin/pin"

install -d "$PREFIX/share/man/man1"
install -m 0644 "$ROOT_DIR/man/man1/pin.1" "$PREFIX/share/man/man1/pin.1"

echo "Installed pin to $PREFIX/bin/pin"
echo "Make sure $PREFIX/bin is on PATH."
