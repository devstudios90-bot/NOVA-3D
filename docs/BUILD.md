# Build Instructions

## Prerequisites
- **Java:** JDK 17 or JDK 21 (`JAVA_HOME` configured)
- **Android SDK & NDK:** `compileSdk 35`, `minSdk 26`, `targetSdk 35`, `ndk;30.0.16248370`, `cmake;3.31.6`, Gradle `9.6.0`
- **Rust:** Stable toolchain (zero external crates required in `rust/nova_core/Cargo.toml`) with Android targets:
  - `aarch64-linux-android`
  - `armv7-linux-androideabi`
  - `x86_64-linux-android`
- **cargo-ndk:** `cargo install cargo-ndk --locked`
- **Google Filament (Android Native C++):** `v1.80.1` prebuilts extracted to `third_party/filament/filament/`
  - Download `filament-v1.80.1-android-native.tgz` from Google Filament Releases and extract into `third_party/filament/`.

## 1. Run Rust Core Unit Tests
```powershell
cargo test --manifest-path rust/nova_core/Cargo.toml
```

## 2. Build Android Debug APK (builds Rust + C++ + Kotlin)
```powershell
cd android
.\gradlew.bat assembleDebug --no-daemon --console=plain
```
The `buildRustCore` Gradle task automatically invokes `cargo ndk` for `arm64-v8a`, `armeabi-v7a`, and `x86_64` before `preBuild`, and CMake links `libnova_core.a` together with Filament's static archives into `libnova_renderer.so`. Output APK: `android/app/build/outputs/apk/debug/app-debug.apk`.

## 3. Install & Launch on Device
```powershell
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -W -n com.nova3d.studio/.MainActivity
```
