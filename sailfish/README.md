# Sailfish OS

Scripts for building and running tasku_peatus on Sailfish OS (aarch64). The build always
enables the `geoclue` feature (location from GeoClue 1) and `scaled` (a fixed 2.375 scale
factor for the phone's display).

The app is cross-compiled on the host with `clang` and `lld` against a sysroot taken from
the Sailfish SDK's aarch64 target; the SDK's own Rust toolchain is too old for this
project. The result is a plain binary that runs over SSH. There is no RPM yet.

## Requirements

- [Sailfish SDK](https://docs.sailfishos.org/Tools/Sailfish_SDK/) with the aarch64 target
  installed (`sfdk tools list`).
- `rustup target add aarch64-unknown-linux-gnu`
- `clang`, `lld`, `llvm` (for `llvm-strip`) and `bsdtar` (`libarchive`).
- A phone with developer mode and **Remote connection** enabled, connected over USB, and
  your SSH key installed: `ssh-copy-id defaultuser@192.168.2.15`. The address is the
  **USB IP address** shown in the phone's developer settings.

## Usage

```sh
sailfish/setup-sysroot.sh   # once, and after updating the SDK target
sailfish/build.sh           # debug build, or --release
sailfish/deploy.sh --run    # copy to the phone and run, Ctrl+C to stop
sailfish/deploy.sh --remove # delete it from the phone
```

`build.sh` writes a stripped binary to `target/sailfish/tasku_peatus`; extra arguments go
to `cargo`. `deploy.sh` copies it to `~/tasku_peatus` on the phone, replacing the previous
copy. With `--run` the window opens on the phone and the logs print in your terminal.

## Configuration

All scripts read [`env.sh`](env.sh); override any of these from the environment:

| Variable | Default |
|---|---|
| `SFOS_HOST` | `defaultuser@192.168.2.15` |
| `SFOS_TARGET` | `SailfishOS-5.1.0.11-aarch64` |
| `SFOS_SYSROOT` | `~/.local/share/sailfish-sysroot/$SFOS_TARGET` |
| `SFOS_WORKSPACE` | The SDK's shared source directory, read from its config |
| `SFDK` | `~/SailfishOS/bin/sfdk` |
| `RUST_LOG` | `tasku_peatus=debug,warn` (for `deploy.sh --run`) |

## Location

GeoClue 1's master client only uses a provider once that provider reports itself
available. For the cell-id provider (`Mlsdb`) that can take minutes, and it drops back to
"acquiring" after about a minute. Positions from the master are therefore sparse and late.
To work around it, the app:

- Asks the providers directly for their cached fix at startup and uses the most accurate
  one less than 15 minutes old, so a position usually appears immediately.
- Requests any accuracy, so network location can be used before GPS has a fix.
- Reads `enabled=` from `/etc/location/location.conf` every 2 seconds to show when
  location is switched off; GeoClue itself gives no dependable signal for that.

## Sysroot

`setup-sysroot.sh` exports `lib64`, `usr/lib64` and `usr/include` from the build engine,
then adds what the target is missing for linking:

- GCC's startup objects and `libgcc` from the `gcc` and `libstdc++-devel` RPMs, which are
  downloaded but not installed, so the SDK target is left untouched.
- Unversioned `libgcc_s.so` and `libwayland-egl.so` links.
- An empty `libGL.so` stub. Skia asks for desktop GL, which Sailfish doesn't ship; the
  linker drops it again because nothing uses it.

The SDK target's glibc must not be newer than the phone's. SDK 5.1.0.11 and Sailfish OS
5.2 both use glibc 2.41.

## Known issues

- **Software rendering.** EGL rejects a config attribute Freya requests
  (`EGL_BAD_ATTRIBUTE`, 3004), so the app falls back to the software renderer.
- **No Sailjail.** Running from SSH bypasses the sandbox. A packaged app will need a
  Sailjail profile granting `Location`.

## Planned

- `.desktop` file and icon.
- RPM packaging with a Sailjail profile.
