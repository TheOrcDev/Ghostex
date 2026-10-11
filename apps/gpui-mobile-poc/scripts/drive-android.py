#!/usr/bin/env python3
"""Drive the POC app on an emulator or USB phone over adb (no GUI automation needed).

  scripts/drive-android.py labels             list on-screen accessibility labels with bounds
  scripts/drive-android.py tap <label>        tap the element with that label (content-desc or text)
  scripts/drive-android.py type "<text>"      type into the focused field
  scripts/drive-android.py shot <file.png>    save a screenshot

The GPUI view is one surface, so rows inside the transcript have no labels; tap them by coordinate
with `adb shell input tap x y` and scroll with `adb shell input swipe x1 y1 x2 y2 <ms>` (a short
duration is a fling). ANDROID_SERIAL picks the device.
"""
import os, re, subprocess, sys, xml.etree.ElementTree as ET

SDK = os.environ.get("ANDROID_HOME", "/opt/homebrew/share/android-commandlinetools")
ADB = [f"{SDK}/platform-tools/adb"] + (["-s", os.environ["ANDROID_SERIAL"]] if "ANDROID_SERIAL" in os.environ else [])


def adb(*args, text=True):
    return subprocess.run(ADB + list(args), capture_output=True, text=text)


def nodes():
    adb("shell", "uiautomator", "dump", "/sdcard/ui.xml")
    xml = adb("exec-out", "cat", "/sdcard/ui.xml").stdout
    return ET.fromstring(xml[xml.index("<"):]).iter("node")


def find(label):
    for node in nodes():
        if label in (node.get("content-desc"), node.get("text")):
            x1, y1, x2, y2 = map(int, re.findall(r"\d+", node.get("bounds")))
            return (x1 + x2) // 2, (y1 + y2) // 2
    return None


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    command, *rest = sys.argv[1:]
    if command == "tap":
        position = find(rest[0]) or sys.exit(f"not found: {rest[0]}")
        adb("shell", "input", "tap", str(position[0]), str(position[1]))
        print("tapped", rest[0], position)
    elif command == "type":
        adb("shell", "input", "text", rest[0].replace(" ", "%s"))
    elif command == "shot":
        with open(rest[0], "wb") as out:
            out.write(adb("exec-out", "screencap", "-p", text=False).stdout)
        print("saved", rest[0])
    elif command == "labels":
        for node in nodes():
            label = node.get("content-desc") or node.get("text")
            if label:
                print(repr(label[:60]), node.get("bounds"))
    else:
        sys.exit(__doc__)


main()
