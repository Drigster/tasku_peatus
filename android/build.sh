#!/usr/bin/env bash
# Builds the Android APK (Rust library included) and copies it to target/android.
# Usage: build.sh [--release]
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

VARIANT=debug
if [[ "${1:-}" == "--release" ]]; then
    VARIANT=release
fi

[[ -d "$ANDROID_HOME" ]] || die "Android SDK not found at $ANDROID_HOME (set ANDROID_HOME)"
[[ -d "$ANDROID_NDK_HOME" ]] || die "Android NDK not found (set ANDROID_NDK_HOME)"
command -v cargo-ndk >/dev/null || die "cargo-ndk is required (cargo install cargo-ndk)"

# The Gradle preBuild step runs cargo-ndk, which always builds the Rust
# library in release mode for both ABIs.
(cd "$GRADLE_DIR" && ./gradlew "assemble${VARIANT^}")

mkdir -p "$(dirname "$APK")"
cp "$GRADLE_DIR/app/build/outputs/apk/$VARIANT/app-$VARIANT.apk" "$APK"
echo "Built $APK ($(du -h "$APK" | cut -f1), $VARIANT)"
