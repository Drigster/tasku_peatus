# Android

The Gradle project and scripts for building and running tasku_peatus on Android. The
Gradle build compiles the Rust library with `cargo-ndk` for `arm64-v8a` and `x86_64` and
packages it with the Kotlin helpers in `app/`, which handle permissions, location updates
and system bar sizes.

> [!WARNING]
> Soft keyboard and IME input are not implemented, so text input does not work.

## Requirements

- Android SDK with API level 36, from [Android Studio](https://developer.android.com/studio)
  or the [command-line tools](https://developer.android.com/studio#command-line-tools-only).
- Android NDK r26 (installable from the SDK manager).
- `cargo install cargo-ndk`
- `rustup target add aarch64-linux-android x86_64-linux-android`
- A device with USB debugging enabled, connected and authorized: `adb devices` must show
  `device`, not `unauthorized`. Accept the prompt on the phone if it shows the latter.

## Usage

```sh
android/build.sh            # debug APK, or --release
android/deploy.sh --run     # install, launch and follow the logs, Ctrl+C to stop
android/deploy.sh --remove  # uninstall from the device
```

`build.sh` copies the APK to `target/android/tasku_peatus.apk`, replacing the previous
build of either variant. `deploy.sh` installs it over the existing install; with `--run`
it restarts the app and follows its logs. Rust logs use the `tasku_peatus` tag and Kotlin
crashes appear under `AndroidRuntime`. Ctrl+C stops following the logs, not the app.

Installing can take about 30 seconds while the phone verifies the APK. Turning off
**Verify apps over USB** in the developer options makes it much faster.

## Release builds

The Rust library is always compiled in release mode, so `--release` only changes the
Android packaging. The release variant is still signed with the machine's debug key:

- It installs over a debug build and the other way round.
- Google Play will not accept it, and a build from another machine cannot update it,
  since each machine has its own debug key.

Publishing needs a real signing key, configured outside the repository.

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
