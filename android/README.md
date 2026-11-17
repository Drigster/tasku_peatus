# Android

The Gradle project and scripts for building and running tasku_peatus on Android. The
Gradle build compiles the Rust library with `cargo-ndk` for `arm64-v8a` and `x86_64` and
packages it with the Kotlin helpers in `app/`.

> [!WARNING]
> Android support is experimental. Soft keyboard and IME input are not implemented yet,
> so text input does not work.

## Requirements

- Android SDK with API level 36, from [Android Studio](https://developer.android.com/studio)
  or the [command-line tools](https://developer.android.com/studio#command-line-tools-only).
- Android NDK r26 (installable from the SDK manager).
- `cargo install cargo-ndk`
- `rustup target add aarch64-linux-android x86_64-linux-android`
- A device with USB debugging enabled, connected and authorized (`adb devices` shows
  `device`, not `unauthorized`).

## Usage

```sh
android/build.sh            # debug APK, or --release
android/deploy.sh --run     # install, launch and follow the logs, Ctrl+C to stop
android/deploy.sh --remove  # uninstall from the device
```

`build.sh` copies the APK to `target/android/tasku_peatus.apk`. `deploy.sh` installs it,
replacing the existing install; with `--run` it restarts the app and streams its logs
(Rust logs use the `tasku_peatus` tag, Kotlin crashes `AndroidRuntime`).

The release variant is signed with the debug key, so it can be installed over a debug
build and vice versa.

## Configuration

All scripts read [`env.sh`](env.sh); override any of these from the environment:

| Variable | Default |
|---|---|
| `ANDROID_HOME` | `sdk.dir` from `local.properties`, else `~/Android/Sdk` |
| `ANDROID_NDK_HOME` | The newest NDK under `$ANDROID_HOME/ndk` |
| `ANDROID_SERIAL` | Unset; set it to choose a device when several are connected |
| `ADB` | `$ANDROID_HOME/platform-tools/adb` |

## Android Studio

Opening `android/` in Android Studio and pressing **Run** also works; it runs the same
Gradle build.

## Attribution

Android support started from Freya's Android example, which was inspired by an earlier
attempt by [rebecca-src](https://github.com/rebecca-src/freya/tree/experiment/android/examples/android).
