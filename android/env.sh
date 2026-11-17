# Shared settings for the Android scripts. Sourced, not executed.
# Every value can be overridden from the environment. To pick a device when
# several are connected, set ANDROID_SERIAL (read by adb itself).

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GRADLE_DIR="$PROJECT_DIR/android"
PACKAGE=dev.drigster.taskupeatus
APK="$PROJECT_DIR/target/android/tasku_peatus.apk"

if [[ -z "${ANDROID_HOME:-}" ]]; then
    ANDROID_HOME="$(sed -n 's/^sdk\.dir=//p' "$GRADLE_DIR/local.properties" 2>/dev/null)"
    ANDROID_HOME="${ANDROID_HOME:-$HOME/Android/Sdk}"
fi
export ANDROID_HOME
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
    ANDROID_NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -n1)"
fi
export ANDROID_NDK_HOME
ADB="${ADB:-$ANDROID_HOME/platform-tools/adb}"

die() {
    echo "error: $*" >&2
    exit 1
}
