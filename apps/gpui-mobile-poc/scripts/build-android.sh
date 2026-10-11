#!/usr/bin/env bash
# Builds libghostex_gpui_mobile.so (packages/gpui-mobile/host) for Android and stages it in the
# gpui-view Expo module's jniLibs, where Gradle packages it into the APK.
#
# Usage: scripts/build-android.sh [--release] [--abi arm64-v8a[,x86_64,...]]
#
# Re-runs are incremental (cargo) and the .so is only copied when it changed, so an unchanged
# build costs a cargo no-op. Environment overrides: ANDROID_HOME, ANDROID_NDK_HOME,
# GPUI_ANDROID_PLATFORM (minimum API level to link against, default 26: the `ndk` crate's
# `nativewindow` feature links libnativewindow, which Android only ships from API 26).
set -euo pipefail

POC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The Rust workspace (host and chat crates) the app loads.
RUST="$(cd "$POC/../../packages/gpui-mobile" && pwd)"
PROFILE=debug
ABIS=arm64-v8a

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) PROFILE=release ;;
    --debug) PROFILE=debug ;;
    --abi) ABIS="$2"; shift ;;
    --abi=*) ABIS="${1#--abi=}" ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
# React Native 0.86 builds with NDK 27.1; link the Rust library with the same one.
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.1.12297006}"
PLATFORM="${GPUI_ANDROID_PLATFORM:-26}"

command -v cargo-ndk >/dev/null || { echo "cargo-ndk is missing: cargo install cargo-ndk" >&2; exit 1; }
[[ -d "$ANDROID_NDK_HOME" ]] || { echo "NDK not found at $ANDROID_NDK_HOME" >&2; exit 1; }

# A target dir of its own: the workspace's host builds (the chat crate) never wait on this lock.
export CARGO_TARGET_DIR="$RUST/target/ndk"
# Full debug info makes a several-hundred-MB debug .so that Gradle then strips anyway; line tables
# keep symbolized panics and backtraces.
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}"

JNI_LIBS="$POC/app/modules/gpui-view/android/src/main/jniLibs"
ASSETS="$POC/app/modules/gpui-view/android/src/main/assets"

# Colour emoji: Android 13+ ships a COLR v1 emoji font, which GPUI's text stack (swash) cannot
# draw, so the APK carries the CBDT Noto Color Emoji the desktop's Ghostty build already vendors.
EMOJI_SRC="$POC/../../.dependencies/ghostty/src/font/res/NotoColorEmoji.ttf"
if [[ -f "$EMOJI_SRC" ]]; then
  mkdir -p "$ASSETS/fonts"
  cmp -s "$EMOJI_SRC" "$ASSETS/fonts/NotoColorEmoji.ttf" || cp "$EMOJI_SRC" "$ASSETS/fonts/NotoColorEmoji.ttf"
else
  echo "warning: $EMOJI_SRC is missing (ghostty submodule not checked out); emoji will not render" >&2
fi
cargo_args=(build -p ghostex-gpui-mobile)
[[ "$PROFILE" == release ]] && cargo_args+=(--release)

IFS=',' read -r -a abi_list <<<"$ABIS"
for abi in "${abi_list[@]}"; do
  case "$abi" in
    arm64-v8a) triple=aarch64-linux-android ;;
    x86_64) triple=x86_64-linux-android ;;
    armeabi-v7a) triple=armv7-linux-androideabi ;;
    x86) triple=i686-linux-android ;;
    *) echo "unsupported ABI: $abi" >&2; exit 2 ;;
  esac
  echo "==> $abi ($PROFILE, API $PLATFORM)"
  (cd "$RUST" && cargo ndk -t "$abi" --platform "$PLATFORM" "${cargo_args[@]}")
  built="$CARGO_TARGET_DIR/$triple/$PROFILE/libghostex_gpui_mobile.so"
  staged="$JNI_LIBS/$abi/libghostex_gpui_mobile.so"
  mkdir -p "$JNI_LIBS/$abi"
  if [[ ! -f "$staged" ]] || ! cmp -s "$built" "$staged"; then
    cp "$built" "$staged"
    echo "    staged $(du -h "$staged" | cut -f1) -> ${staged#$POC/}"
  else
    echo "    unchanged ($(du -h "$staged" | cut -f1))"
  fi
done
