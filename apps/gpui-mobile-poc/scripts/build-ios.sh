#!/usr/bin/env bash
# Builds libghostex_gpui_mobile.a (packages/gpui-mobile/host) for iOS and packages it, with its C
# header and module map (host/src/ios/include), as GhostexGpuiMobile.xcframework in the gpui-view
# Expo module, whose podspec vendors it.
#
# Usage: scripts/build-ios.sh [--release] [--device]
#
#   --release   the release profile (thin LTO); default is the dev profile.
#   --device    also build the device slice (aarch64-apple-ios); default is the simulator only
#               (aarch64-apple-ios-sim, Apple silicon).
#
# Re-runs are incremental (cargo), and the XCFramework is only rewritten when a library or the
# header changed, so an unchanged build costs a cargo no-op and CocoaPods sees no change.
set -euo pipefail

POC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The Rust workspace (host and chat crates) the app links.
RUST="$(cd "$POC/../../packages/gpui-mobile" && pwd)"
PROFILE=debug
DEVICE=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) PROFILE=release ;;
    --debug) PROFILE=debug ;;
    --device) DEVICE=1 ;;
    -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

# A target dir of its own: never waits on the Android build's (target/ndk) or the host's lock.
export CARGO_TARGET_DIR="$RUST/target/ios"
# Full debug info makes a multi-GB debug archive; line tables keep symbolized panics.
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}"
# The app's minimum iOS (the podspec's platform); objects built for a newer one make the linker warn.
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-16.4}"

triples=(aarch64-apple-ios-sim)
[[ "$DEVICE" == 1 ]] && triples+=(aarch64-apple-ios)

libs=()
for triple in "${triples[@]}"; do
  echo "==> $triple ($PROFILE)"
  # Only the static library: the crate is also a cdylib (Android) and an rlib, which iOS needs
  # neither of.
  args=(rustc -p ghostex-gpui-mobile --lib --crate-type staticlib --target "$triple")
  [[ "$PROFILE" == release ]] && args+=(--release)
  (cd "$RUST" && cargo "${args[@]}")
  lib="$CARGO_TARGET_DIR/$triple/$PROFILE/libghostex_gpui_mobile.a"
  echo "    $(du -h "$lib" | cut -f1) ${lib#$RUST/}"
  libs+=("$lib")
done

HEADERS_SRC="$RUST/host/src/ios/include"
XCFRAMEWORK="$POC/app/modules/gpui-view/ios/GhostexGpuiMobile.xcframework"
STAMP="$CARGO_TARGET_DIR/xcframework.stamp"

# Rewrite the XCFramework only when an input changed since it was last built (cargo rewrites a
# library only when it changed, so path, size and modification time identify it).
inputs=("${libs[@]}" "$HEADERS_SRC/ghostex_gpui.h" "$HEADERS_SRC/module.modulemap")
fingerprint="$(for input in "${inputs[@]}"; do stat -f '%N %z %m' "$input"; done)"
if [[ -d "$XCFRAMEWORK" && "$(cat "$STAMP" 2>/dev/null)" == "$fingerprint" ]]; then
  echo "==> $(basename "$XCFRAMEWORK") unchanged"
  exit 0
fi

HEADERS="$CARGO_TARGET_DIR/xcframework-headers"
rm -rf "$HEADERS"
mkdir -p "$HEADERS"
cp "$HEADERS_SRC/ghostex_gpui.h" "$HEADERS_SRC/module.modulemap" "$HEADERS/"

xc_args=(-create-xcframework)
for lib in "${libs[@]}"; do
  xc_args+=(-library "$lib" -headers "$HEADERS")
done
rm -rf "$XCFRAMEWORK"
xc_args+=(-output "$XCFRAMEWORK")
xcodebuild "${xc_args[@]}" >/dev/null
echo "$fingerprint" >"$STAMP"
echo "==> wrote ${XCFRAMEWORK#$POC/} ($(du -sh "$XCFRAMEWORK" | cut -f1))"
