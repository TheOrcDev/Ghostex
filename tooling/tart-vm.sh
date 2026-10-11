#!/bin/bash
set -euo pipefail

tart=""
for candidate in "$HOME/.local/share/ghostex/vm-tools/tart" "$HOME"/.local/share/ghostex/*/tart-tools/tart.app/Contents/MacOS/tart "$(command -v tart || true)"; do
  if [[ -x "$candidate" ]]; then
    # AppKit locates the VM window through the executable's real .app bundle.
    tart="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$candidate")"
    break
  fi
done
[[ -n "$tart" ]] || { echo "Tart not found. Install it with brew install cirruslabs/cli/tart." >&2; exit 1; }
vm_name="macos-sandbox"
vm_state="$("$tart" list --format json | python3 -c 'import json,sys; print(next((vm["State"] for vm in json.load(sys.stdin) if vm["Name"] == "macos-sandbox"), "missing"))')"

case "${1:-}" in
  start)
    if [[ "$vm_state" == running ]]; then
      echo "Tart is already running."
      exit 0
    fi
    echo "Starting or resuming Tart. Use Sleep Tart to save the VM and free its memory."
    exec "$tart" run --suspendable "$vm_name"
    ;;
  sleep)
    if [[ "$vm_state" == suspended || "$vm_state" == stopped ]]; then
      echo "Tart is already $vm_state."
      exit 0
    fi
    "$tart" suspend "$vm_name"
    for ((attempt = 0; attempt < 60; attempt++)); do
      vm_state="$("$tart" list --format json | python3 -c 'import json,sys; print(next(vm["State"] for vm in json.load(sys.stdin) if vm["Name"] == "macos-sandbox"))')"
      if [[ "$vm_state" == suspended ]]; then
        echo "Tart is asleep. Start / Resume Tart will restore the open apps."
        exit 0
      fi
      sleep 1
    done
    echo "Tart has not finished suspending (state: $vm_state)." >&2
    exit 1
    ;;
  *)
    echo "Usage: $0 start|sleep" >&2
    exit 2
    ;;
esac
