#!/usr/bin/env bash
# Builds the GPUI mobile proof of concept for the iOS simulator, installs it on a simulator of its
# own (created on first use, booted headless) and launches it.
#
# Usage: scripts/run-ios.sh [--release] [--simulator <name>] [--no-launch] [--skip-rust]
#                           [--skip-app] [-- <launch arguments>]
#
#   --release       build the Rust library with the release profile (thin LTO). The app is always
#                   a Release build with the JS bundle embedded, so no Metro server is needed.
#   --simulator     simulator to use (default: $GPUI_IOS_SIMULATOR, else "gpui-poc-iphone", an
#                   iPhone 17 Pro on the newest iOS runtime; other simulators are never touched).
#   --no-launch     install only.
#   --skip-rust     reuse the staged XCFramework.
#   --skip-app      reuse the last built .app (relaunch with other arguments).
#   -- ...          arguments for the app, e.g. `-- --gpuiSession <projectId:sessionId>` or
#                   `-- --gpuiContent demo` (the Android intent extras); `--gpuiTestHook 1`
#                   enables the test remote control (modules/gpui-view/ios/GpuiTestHook.swift).
#
# Idempotent: node_modules, the generated ios/ folder and the Pods are only (re)created when
# missing or when app.json / package.json / the module's podspec changed; cargo and xcodebuild are
# incremental. The app's standard output and error (Rust and Swift logs) go to
# packages/gpui-mobile/target/ios/logs/app.log; Rust also writes <app data>/Library/Application Support/gpui/logs/gpui.log.
# For JS iteration use Metro instead: a Debug build (`cd app && bunx expo run:ios`) and
# `bunx expo start`.
set -euo pipefail

POC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$POC/app"
RUST_ARGS=()
SIMULATOR="${GPUI_IOS_SIMULATOR:-gpui-poc-iphone}"
DEVICE_TYPE="iPhone 17 Pro"
LAUNCH=1
SKIP_RUST=0
SKIP_APP=0
LAUNCH_ARGS=()
BUNDLE_ID=dev.ghostex.gpuipoc

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RUST_ARGS+=(--release) ;;
    --simulator) SIMULATOR="$2"; shift ;;
    --no-launch) LAUNCH=0 ;;
    --skip-rust) SKIP_RUST=1 ;;
    --skip-app) SKIP_APP=1; SKIP_RUST=1 ;;
    --) shift; LAUNCH_ARGS=("$@"); break ;;
    -h|--help) sed -n '2,25p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

# The simulator: this script's own, created once, booted without opening Simulator.app.
udid="$(xcrun simctl list devices -j | python3 -c '
import json, sys
name = sys.argv[1]
for runtime, devices in json.load(sys.stdin)["devices"].items():
    for device in devices:
        if device["name"] == name and device.get("isAvailable", True):
            print(device["udid"]); sys.exit()
' "$SIMULATOR")"
if [[ -z "$udid" ]]; then
  runtime="$(xcrun simctl list runtimes -j | python3 -c '
import json, sys
ios = [r for r in json.load(sys.stdin)["runtimes"] if r["platform"] == "iOS" and r["isAvailable"]]
print(sorted(ios, key=lambda r: [int(p) for p in r["version"].split(".")])[-1]["identifier"])')"
  echo "==> creating simulator $SIMULATOR ($DEVICE_TYPE, $runtime)"
  udid="$(xcrun simctl create "$SIMULATOR" "$DEVICE_TYPE" "$runtime")"
fi
if ! xcrun simctl list devices | grep -q "$udid) (Booted)"; then
  echo "==> booting $SIMULATOR ($udid)"
  xcrun simctl boot "$udid"
fi
xcrun simctl bootstatus "$udid" -b >/dev/null

BUILD="$APP/ios/build"
APP_BUNDLE_GLOB="$BUILD/Build/Products/Release-iphonesimulator/*.app"

if [[ "$SKIP_APP" == 0 ]]; then
  # 1. JS dependencies.
  if [[ ! -d "$APP/node_modules" || "$APP/package.json" -nt "$APP/node_modules/.bun-installed" ]]; then
    echo "==> bun install"
    (cd "$APP" && bun install && touch node_modules/.bun-installed)
  fi

  # 2. The generated native project (Continuous Native Generation; ios/ is gitignored).
  STAMP="$APP/ios/.prebuild-stamp"
  if [[ ! -f "$STAMP" || "$APP/app.json" -nt "$STAMP" || "$APP/package.json" -nt "$STAMP" ]]; then
    echo "==> expo prebuild --platform ios"
    (cd "$APP" && CI=1 bunx expo prebuild --platform ios --no-install)
    touch "$STAMP"
  fi

  # 3. The gxserver endpoint the app bundles. The simulator shares the computer's network, so
  # http://127.0.0.1:<port> is gxserver itself (no adb reverse: ANDROID_HOME is pointed away).
  ANDROID_HOME=/nonexistent "$POC/scripts/dev-endpoint.sh"

  # 4. The Rust library, packaged into the Expo module's XCFramework.
  if [[ "$SKIP_RUST" == 0 ]]; then
    "$POC/scripts/build-ios.sh" "${RUST_ARGS[@]+"${RUST_ARGS[@]}"}"
  fi
  [[ -d "$APP/modules/gpui-view/ios/GhostexGpuiMobile.xcframework" ]] || {
    echo "GhostexGpuiMobile.xcframework is missing: run scripts/build-ios.sh" >&2
    exit 1
  }

  # 5. CocoaPods, when the Podfile, the module's podspec or the XCFramework's slices changed
  # (the Pods project copies only the slices the XCFramework had at `pod install`, so a device
  # slice added by `build-ios.sh --device` needs a new install before a device build).
  PODS_STAMP="$APP/ios/Pods/.pods-stamp"
  XCFRAMEWORK_PLIST="$APP/modules/gpui-view/ios/GhostexGpuiMobile.xcframework/Info.plist"
  if [[ ! -f "$PODS_STAMP" || "$APP/ios/Podfile" -nt "$PODS_STAMP" \
    || "$APP/modules/gpui-view/ios/GpuiView.podspec" -nt "$PODS_STAMP" \
    || "$APP/package.json" -nt "$PODS_STAMP" \
    || "$(cat "$APP/ios/Pods/.xcframework-slices" 2>/dev/null)" != "$(plutil -extract AvailableLibraries xml1 -o - "$XCFRAMEWORK_PLIST" | grep -A1 LibraryIdentifier | grep string)" ]]; then
    echo "==> pod install"
    # Pod downloads report progress on standard error; keep only the result and any failure.
    (cd "$APP/ios" && pod install 2>&1 | { grep -vE '^ +[0-9]+ |% Total|Dload|script phase|^$' || true; }
      exit "${PIPESTATUS[0]}")
    touch "$PODS_STAMP"
    plutil -extract AvailableLibraries xml1 -o - "$XCFRAMEWORK_PLIST" | grep -A1 LibraryIdentifier \
      | grep string >"$APP/ios/Pods/.xcframework-slices"
  fi

  # 6. A Release build for the simulator: the JS bundle is embedded, so no Metro is needed. Only
  # the simulator's own architecture (arm64): the XCFramework has no x86_64 slice.
  workspace="$(ls -d "$APP"/ios/*.xcworkspace | head -1)"
  scheme="$(basename "$workspace" .xcworkspace)"
  echo "==> xcodebuild $scheme (Release, simulator)"
  xcodebuild -workspace "$workspace" -scheme "$scheme" -configuration Release \
    -sdk iphonesimulator -destination "id=$udid" -derivedDataPath "$BUILD" \
    ONLY_ACTIVE_ARCH=YES CODE_SIGNING_ALLOWED=NO build -quiet
fi

app_bundle="$(ls -d $APP_BUNDLE_GLOB | head -1)"
echo "    $(du -sh "$app_bundle" | cut -f1) $app_bundle"

# 7. Install and launch.
echo "==> install on $SIMULATOR"
xcrun simctl install "$udid" "$app_bundle"
if [[ "$LAUNCH" == 1 ]]; then
  LOGS="$POC/../../packages/gpui-mobile/target/ios/logs"
  mkdir -p "$LOGS"
  # simctl appends to these files; each launch starts a fresh log.
  : >"$LOGS/app.log"
  xcrun simctl launch --terminate-running-process \
    --stdout="$LOGS/app.log" --stderr="$LOGS/app.log" \
    "$udid" "$BUNDLE_ID" "${LAUNCH_ARGS[@]+"${LAUNCH_ARGS[@]}"}"
  echo "==> launched $BUNDLE_ID on $SIMULATOR (logs: $LOGS/app.log; screenshot:"
  echo "    xcrun simctl io $udid screenshot shot.png)"
fi
