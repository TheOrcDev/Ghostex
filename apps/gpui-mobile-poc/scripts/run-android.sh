#!/usr/bin/env bash
# Builds the GPUI mobile proof of concept and installs it on an Android emulator or device.
#
# Usage: scripts/run-android.sh [--release] [--serial <adb serial>] [--no-launch] [--skip-rust]
#
#   --release     build the Rust library with the release profile (thin LTO). The APK is always
#                 a release APK with the JS bundle embedded, so no Metro server is needed.
#   --serial      adb device to install on (default: $ANDROID_SERIAL, else the only device).
#   --no-launch   install only.
#   --skip-rust   reuse the staged .so.
#
# Idempotent: node_modules and the generated android/ folder are only (re)created when missing or
# when app.json / package.json changed since the last prebuild; cargo and Gradle are incremental.
# For JS iteration without rebuilding the APK, use Metro instead: `cd app && bunx expo start`,
# `adb reverse tcp:8081 tcp:8081`, and a debug APK (`./gradlew installDebug` in app/android).
set -euo pipefail

POC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$POC/app"
RUST_ARGS=()
SERIAL="${ANDROID_SERIAL:-}"
LAUNCH=1
SKIP_RUST=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RUST_ARGS+=(--release) ;;
    --serial) SERIAL="$2"; shift ;;
    --no-launch) LAUNCH=0 ;;
    --skip-rust) SKIP_RUST=1 ;;
    -h|--help) sed -n '2,16p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
ADB="$ANDROID_HOME/platform-tools/adb"
[[ -n "$SERIAL" ]] && ADB="$ADB -s $SERIAL"
PACKAGE=dev.ghostex.gpuipoc

# 1. JS dependencies.
if [[ ! -d "$APP/node_modules" || "$APP/package.json" -nt "$APP/node_modules/.bun-installed" ]]; then
  echo "==> bun install"
  (cd "$APP" && bun install && touch node_modules/.bun-installed)
fi

# 2. The generated native project (Continuous Native Generation; android/ is gitignored).
STAMP="$APP/android/.prebuild-stamp"
if [[ ! -f "$STAMP" || "$APP/app.json" -nt "$STAMP" || "$APP/package.json" -nt "$STAMP" ]]; then
  echo "==> expo prebuild"
  (cd "$APP" && CI=1 bunx expo prebuild --platform android --no-install)
  touch "$STAMP"
fi

# 3. The gxserver endpoint the app bundles (and adb reverse to it).
"$POC/scripts/dev-endpoint.sh" ${SERIAL:+--serial "$SERIAL"}

# 4. The Rust library, staged into the Expo module's jniLibs.
if [[ "$SKIP_RUST" == 0 ]]; then
  "$POC/scripts/build-android.sh" "${RUST_ARGS[@]+"${RUST_ARGS[@]}"}"
fi

# 5. A release APK for arm64 only. minSdk 26 because the Rust library links libnativewindow.
echo "==> gradle assembleRelease"
(cd "$APP/android" && ./gradlew assembleRelease \
  -PreactNativeArchitectures=arm64-v8a \
  -Pandroid.minSdkVersion=26 \
  --console=plain -q)
APK="$APP/android/app/build/outputs/apk/release/app-release.apk"
echo "    $(du -h "$APK" | cut -f1) $APK"

# 6. Install and launch.
echo "==> install"
$ADB install -r "$APK" >/dev/null
if [[ "$LAUNCH" == 1 ]]; then
  $ADB shell am start -n "$PACKAGE/.MainActivity" >/dev/null
  echo "==> launched $PACKAGE (logs: adb logcat -s GhostexGpui GpuiView GpuiNative ReactNativeJS)"
fi
