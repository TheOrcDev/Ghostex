#!/usr/bin/env bash
# Writes app/dev-endpoint.json (gitignored) with this computer's gxserver URL and token, which the
# app bundles, and, for every adb device, forwards the phone's 127.0.0.1:<port> to it.
#
# Usage: scripts/dev-endpoint.sh [--session <projectId:sessionId>] [--serial <adb serial>]
#
# The token is read from gxserver's own state and never printed.
set -euo pipefail

POC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE="${GHOSTEX_GXSERVER_STATE:-$HOME/.local/state/ghostex/gxserver}"
SESSION=""
SERIAL="${ANDROID_SERIAL:-}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --session) SESSION="$2"; shift ;;
    --serial) SERIAL="$2"; shift ;;
    -h|--help) sed -n '2,8p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

PORT="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['port'])" "$STATE/runtime/server.json")"
[[ -s "$STATE/auth/token" ]] || { echo "no gxserver token at $STATE/auth/token (is gxserver running?)" >&2; exit 1; }

python3 - "$POC/app/dev-endpoint.json" "$PORT" "$STATE/auth/token" "$SESSION" <<'PY'
import json, sys
out, port, token_path, session = sys.argv[1:5]
data = {"baseUrl": f"http://127.0.0.1:{port}", "authToken": open(token_path).read().strip()}
if session:
    data["session"] = session
text = json.dumps(data, indent=2) + "\n"
try:
    unchanged = open(out).read() == text
except FileNotFoundError:
    unchanged = False
if not unchanged:
    open(out, "w").write(text)
print(f"==> dev endpoint http://127.0.0.1:{port}" + (" (unchanged)" if unchanged else ""))
PY

ADB="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}/platform-tools/adb"
if [[ -x "$ADB" ]]; then
  if [[ -n "$SERIAL" ]]; then serials="$SERIAL"; else serials="$("$ADB" devices | awk 'NR>1 && $2=="device" {print $1}')"; fi
  for serial in $serials; do
    "$ADB" -s "$serial" reverse "tcp:$PORT" "tcp:$PORT" >/dev/null && echo "    adb reverse tcp:$PORT on $serial"
  done
fi
