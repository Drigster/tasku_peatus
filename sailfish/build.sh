#!/usr/bin/env bash
# Cross-compiles tasku_peatus for Sailfish OS (aarch64).
# Usage: build.sh [--release] [extra cargo args...]
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

PROFILE=debug
if [[ "${1:-}" == "--release" ]]; then
    PROFILE=release
    shift
fi

GCC_INSTALL_DIR="$(ls -d "$SFOS_SYSROOT"/usr/lib64/gcc/aarch64-meego-linux-gnu/* 2>/dev/null | head -n1)"
[[ -n "$GCC_INSTALL_DIR" ]] || die "sysroot missing or incomplete, run sailfish/setup-sysroot.sh"

R="$SFOS_SYSROOT"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=clang
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS="\
-C link-arg=--target=aarch64-linux-gnu \
-C link-arg=--sysroot=$R \
-C link-arg=-fuse-ld=lld \
-C link-arg=--gcc-install-dir=$GCC_INSTALL_DIR \
-C link-arg=-L$R/lib64 \
-C link-arg=-L$R/usr/lib64 \
-C link-arg=-Wl,-rpath-link,$R/lib64:$R/usr/lib64"
export CC_aarch64_unknown_linux_gnu=clang
export CXX_aarch64_unknown_linux_gnu=clang++
export CFLAGS_aarch64_unknown_linux_gnu="--target=aarch64-linux-gnu --sysroot=$R"
export CXXFLAGS_aarch64_unknown_linux_gnu="--target=aarch64-linux-gnu --sysroot=$R"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$R"
export PKG_CONFIG_LIBDIR="$R/usr/lib64/pkgconfig:$R/usr/share/pkgconfig"

cd "$PROJECT_DIR"
cargo_args=(build --target "$RUST_TARGET" --features geoclue,scaled)
[[ "$PROFILE" == release ]] && cargo_args+=(--release)
cargo "${cargo_args[@]}" "$@"

OUT="$PROJECT_DIR/target/sailfish"
mkdir -p "$OUT"
llvm-strip -o "$OUT/tasku_peatus" "$PROJECT_DIR/target/$RUST_TARGET/$PROFILE/tasku_peatus"
echo "Built $OUT/tasku_peatus ($(du -h "$OUT/tasku_peatus" | cut -f1), $PROFILE)"
