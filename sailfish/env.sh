# Shared settings for the Sailfish OS scripts. Sourced, not executed.
# Every value can be overridden from the environment.

SFOS_TARGET="${SFOS_TARGET:-SailfishOS-5.1.0.11-aarch64}"
SFOS_SYSROOT="${SFOS_SYSROOT:-$HOME/.local/share/sailfish-sysroot/$SFOS_TARGET}"
SFOS_HOST="${SFOS_HOST:-defaultuser@192.168.2.15}"
SFDK="${SFDK:-$HOME/SailfishOS/bin/sfdk}"

RUST_TARGET=aarch64-unknown-linux-gnu
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# sfdk only runs inside its workspace (the SDK's shared source directory).
sfos_workspace() {
    if [[ -n "${SFOS_WORKSPACE:-}" ]]; then
        echo "$SFOS_WORKSPACE"
        return
    fi
    sed -n 's|.*key="SharedSrc">\(.*\)</value>.*|\1|p' \
        "$HOME/.config/SailfishSDK/libsfdk/buildengines.xml" 2>/dev/null | head -n1
}

die() {
    echo "error: $*" >&2
    exit 1
}
