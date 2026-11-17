#!/usr/bin/env bash
# Installs the built APK on a connected device, optionally running it, or uninstalls it.
# Usage: deploy.sh [--run | --remove]
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

[[ -x "$ADB" ]] || die "adb not found at $ADB (set ADB or ANDROID_HOME)"

case "${1:-}" in
"" | --run) ;;
--remove)
    echo "Uninstalling $PACKAGE"
    exec "$ADB" uninstall "$PACKAGE"
    ;;
*) die "unknown option '$1' (expected --run or --remove)" ;;
esac

[[ -f "$APK" ]] || die "no build found, run android/build.sh first"

echo "Installing $APK (can take ~30 s while the device verifies it)"
"$ADB" install -r "$APK"

if [[ "${1:-}" == "--run" ]]; then
    "$ADB" shell am force-stop "$PACKAGE"
    "$ADB" logcat -c
    "$ADB" shell am start -n "$PACKAGE/.MainActivity" >/dev/null
    # Rust logs go out under one tag; Kotlin crashes show under AndroidRuntime.
    # Ctrl+C stops following the logs (the app keeps running).
    exec "$ADB" logcat -v time -s 'tasku_peatus:*' 'AndroidRuntime:E'
fi
