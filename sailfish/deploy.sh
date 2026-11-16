#!/usr/bin/env bash
# Copies the built binary to the phone, optionally running it, or removes it.
# Usage: deploy.sh [--run | --remove]
# Set SFOS_HOST to change the target (default defaultuser@192.168.2.15, USB networking).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

BINARY="$PROJECT_DIR/target/sailfish/tasku_peatus"

case "${1:-}" in
"" | --run) ;;
--remove)
    echo "Removing tasku_peatus from $SFOS_HOST"
    exec ssh "$SFOS_HOST" 'rm -f ~/tasku_peatus'
    ;;
*) die "unknown option '$1' (expected --run or --remove)" ;;
esac

[[ -f "$BINARY" ]] || die "no build found, run sailfish/build.sh first"

echo "Copying to $SFOS_HOST:tasku_peatus"
scp -q "$BINARY" "$SFOS_HOST:tasku_peatus"

if [[ "${1:-}" == "--run" ]]; then
    # The SSH session already has WAYLAND_DISPLAY and the session bus, so the
    # window opens on the phone. Ctrl+C stops the app.
    exec ssh -t "$SFOS_HOST" "RUST_LOG='${RUST_LOG:-tasku_peatus=debug,warn}' ./tasku_peatus"
fi
