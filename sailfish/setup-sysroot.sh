#!/usr/bin/env bash
# Builds a sysroot for cross-compiling from the Sailfish SDK's build engine.
# Run once, and again after updating the SDK target.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

command -v bsdtar >/dev/null || die "bsdtar is required (pacman -S libarchive)"
command -v clang >/dev/null || die "clang is required"
[[ -x "$SFDK" ]] || die "sfdk not found at $SFDK (set SFDK)"
WORKSPACE="$(sfos_workspace)"
[[ -d "$WORKSPACE" ]] || die "Sailfish SDK workspace not found (set SFOS_WORKSPACE)"

TARGET_ROOT="/srv/mer/targets/$SFOS_TARGET"
GCC_DIR="usr/lib64/gcc/aarch64-meego-linux-gnu"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Creating sysroot for $SFOS_TARGET in $SFOS_SYSROOT"
rm -rf "$SFOS_SYSROOT"
mkdir -p "$SFOS_SYSROOT"
cd "$WORKSPACE"

# sfdk's command output isn't binary-safe, so archives travel as base64.
echo "Exporting libraries and headers from the build engine..."
"$SFDK" engine exec sh -c "tar -cf - -C '$TARGET_ROOT' \
    lib/ld-linux-aarch64.so.1 lib64 usr/lib64 usr/include usr/share/pkgconfig | base64 -w0" \
    | base64 -di | tar -xf - -C "$SFOS_SYSROOT"

# The target has no GCC, but linking needs its startup objects and libgcc.
echo "Fetching GCC startup files..."
"$SFDK" tools exec "$SFOS_TARGET" sh -c '
    d=$(mktemp -d)
    zypper --non-interactive --pkg-cache-dir "$d" download gcc libstdc++-devel >&2
    find "$d" -name "*.rpm" -print0 | tar -cf - --null -T - --transform "s|.*/||" | base64 -w0
    rm -rf "$d"
' 2>"$TMP/zypper.log" | base64 -di >"$TMP/rpms.tar" || { cat "$TMP/zypper.log" >&2; die "downloading GCC failed"; }
tar -xf "$TMP/rpms.tar" -C "$TMP"
for rpm in "$TMP"/*.rpm; do
    bsdtar -xf "$rpm" -C "$SFOS_SYSROOT" "./$GCC_DIR/*" "./usr/include/c++/*" 2>/dev/null || true
done
ls "$SFOS_SYSROOT/$GCC_DIR"/*/crtbeginS.o >/dev/null || die "GCC startup files missing after extraction"

echo "Fixing up links..."
# Absolute symlinks would point at the host's own libraries.
find "$SFOS_SYSROOT" -type l -lname '/*' | while read -r link; do
    target="$(readlink "$link")"
    ln -sfn "$(realpath -m --relative-to="$(dirname "$link")" "$SFOS_SYSROOT$target")" "$link"
done
ln -sf libgcc_s.so.1 "$SFOS_SYSROOT/lib64/libgcc_s.so"
ln -sf libwayland-egl.so.1 "$SFOS_SYSROOT/usr/lib64/libwayland-egl.so"

# Skia asks for desktop GL, which Sailfish doesn't have (it uses GLES). An empty
# stub satisfies the linker; --as-needed then drops it from the binary.
echo '' >"$TMP/empty.c"
clang --target=aarch64-linux-gnu -nostdlib -shared -fuse-ld=lld \
    -Wl,-soname,libGL.so.1 -o "$SFOS_SYSROOT/usr/lib64/libGL.so" "$TMP/empty.c"

echo "Done ($(du -sh "$SFOS_SYSROOT" | cut -f1))."
