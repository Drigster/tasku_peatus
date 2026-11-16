# Sailfish OS

Scripts for building and running tasku_peatus on Sailfish OS (aarch64). The build always
enables two features: `geoclue` (location from GeoClue 1) and `scaled` (a fixed 2.375 scale
factor for the phone's display).

## How it works

The app is cross-compiled on the host with `clang` and `lld`, linking against a sysroot
taken from the Sailfish SDK's aarch64 build target. The SDK's own Rust toolchain is too
old for this project, so `sfdk` is only used to export the target's libraries.

The result is a plain binary. There is no RPM yet (see [Planned](#planned)).

## Requirements

- [Sailfish SDK](https://docs.sailfishos.org/Tools/Sailfish_SDK/) with the aarch64 target
  installed (`sfdk tools list`).
- `rustup target add aarch64-unknown-linux-gnu`
- `clang`, `lld`, `llvm` (for `llvm-strip`) and `bsdtar` (`libarchive`).
- A phone with developer mode and remote connection enabled, and your SSH key installed:
  `ssh-copy-id defaultuser@192.168.2.15`.

## Usage

```sh
sailfish/setup-sysroot.sh   # once, and after updating the SDK target
sailfish/build.sh           # debug build, or --release
sailfish/deploy.sh --run    # copy to the phone and run, Ctrl+C to stop
sailfish/deploy.sh --remove # delete it from the phone
```

`build.sh` writes a stripped binary to `target/sailfish/tasku_peatus`. `deploy.sh` copies
it to `~/tasku_peatus` on the phone, replacing the previous copy; with `--run` the window
opens on the phone and logs print in your terminal. Extra arguments to `build.sh` go to
`cargo`.

## Configuration

All scripts read [`env.sh`](env.sh); override any of these from the environment:

| Variable | Default |
|---|---|
| `SFOS_HOST` | `defaultuser@192.168.2.15` (USB networking address) |
| `SFOS_TARGET` | `SailfishOS-5.1.0.11-aarch64` |
| `SFOS_SYSROOT` | `~/.local/share/sailfish-sysroot/$SFOS_TARGET` |
| `SFOS_WORKSPACE` | The SDK's shared source directory, read from its config |
| `SFDK` | `~/SailfishOS/bin/sfdk` |
| `RUST_LOG` | `tasku_peatus=debug,warn` (for `deploy.sh --run`) |

## Sysroot details

`setup-sysroot.sh` exports `lib64`, `usr/lib64` and `usr/include` from the build engine,
then patches what the target is missing for linking:

- GCC's startup objects and `libgcc` from the `gcc` and `libstdc++-devel` RPMs, which are
  downloaded but not installed, so the SDK target is left untouched.
- Unversioned `libgcc_s.so` and `libwayland-egl.so` links.
- An empty `libGL.so` stub. Skia asks for desktop GL, which Sailfish doesn't ship; the
  linker drops it again because nothing uses it.

The SDK target's glibc must not be newer than the phone's. SDK 5.1.0.11 and the phone's
Sailfish OS 5.2 both use glibc 2.41.

## Known issues

- **Software rendering.** EGL rejects a config attribute Freya requests
  (`EGL_BAD_ATTRIBUTE`, 3004), so the app falls back to the software renderer.
- **First fix takes about 10 seconds.** GeoClue starts its providers on demand; the first
  position comes from network location, then GPS once it has a fix.
- **Location permission.** Running from SSH bypasses Sailjail. A packaged app will need a
  Sailjail profile granting `Location`.

## Planned

- `.desktop` file and icon.
- RPM packaging (`rpm/tasku_peatus.spec`) with a Sailjail profile.
