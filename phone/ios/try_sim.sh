#!/bin/bash
# Backline in the iPhone simulator, with a picture of each screen (target/ios/shots): each is the app started with
# the PC copy's PHONE_* settings (src/lib.rs, pc_hooks), passed through simctl's SIMCTL_CHILD_ ones. The app's log,
# what it printed and any crash report go beside the pictures.
set -u
cd "$(dirname "$0")/.."
OUT="$PWD/target/ios"; SH="$OUT/shots"; rm -rf "$SH"; mkdir -p "$SH"
APP="$OUT/Backline-sim.app"; ID=com.v9studio.backline.practice
TYPE=$(xcrun simctl list devicetypes | grep -oE "com\.apple\.CoreSimulator\.SimDeviceType\.iPhone-1[0-9](-Pro)?\b" | grep -v Max | tail -1)
RT=$(xcrun simctl list runtimes available | grep -oE "com\.apple\.CoreSimulator\.SimRuntime\.iOS-[0-9-]+" | tail -1)
echo "simulator: $TYPE on $RT"
DEV=$(xcrun simctl create backline "$TYPE" "$RT")
xcrun simctl boot "$DEV"
xcrun simctl bootstatus "$DEV" -b > /dev/null
xcrun simctl install "$DEV" "$APP"
DATA=$(xcrun simctl get_app_container "$DEV" $ID data)
OWN="$DATA/Library/Application Support/Backline"
mkdir -p "$OWN" "$DATA/Documents/Standards"
touch "$OWN/MUTE"
# (songs in Backline's Files folder: a few seconds of a chord, made here)
python3 - "$DATA/Documents" <<'PY'
import math, struct, sys, wave
def tone(path, freqs, secs=6):
    with wave.open(path, "wb") as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(44100)
        fr = b"".join(struct.pack("<hh", v, v) for v in (int(6000 * sum(math.sin(2 * math.pi * f * i / 44100) for f in freqs) / len(freqs)) for i in range(44100 * secs)))
        w.writeframes(fr)
d = sys.argv[1]
tone(d + "/Wayne Shorter - Juju (practice).wav", [220, 277.2, 329.6])
tone(d + "/Standards/Autumn Leaves.wav", [196, 246.9, 293.7])
tone(d + "/Standards/Blue Bossa.wav", [261.6, 311.1, 392])
PY
shot() {
  local name=$1 wait=$2; shift 2
  xcrun simctl terminate "$DEV" $ID > /dev/null 2>&1
  env "$@" xcrun simctl launch --stdout="$SH/$name.out.txt" --stderr="$SH/$name.err.txt" "$DEV" $ID > /dev/null
  sleep "$wait"
  xcrun simctl io "$DEV" screenshot "$SH/$name.png" > /dev/null 2>&1 || echo "no picture of $name"
}
shot 1-start 10
shot 2-files 6 SIMCTL_CHILD_PHONE_VIEW=mine SIMCTL_CHILD_PHONE_MINE=2
shot 3-folder 6 SIMCTL_CHILD_PHONE_VIEW=mine SIMCTL_CHILD_PHONE_MINE=2 SIMCTL_CHILD_PHONE_FOLDER=Standards
shot 4-settings 6 SIMCTL_CHILD_PHONE_VIEW=settings
shot 5-search 14 SIMCTL_CHILD_PHONE_VIEW=songs "SIMCTL_CHILD_PHONE_QUERY=juju wayne shorter"
shot 6-youtube 45 SIMCTL_CHILD_PHONE_VIEW=practice SIMCTL_CHILD_PHONE_SONG=soJvqZHSHxk SIMCTL_CHILD_PHONE_PLAY=1
shot 7-metronome 8 SIMCTL_CHILD_PHONE_METRO=on
cp "$OWN/log.txt" "$SH/log.txt" 2>/dev/null || echo "(no log)" > "$SH/log.txt"
ls -la "$DATA/Documents" "$OWN" > "$SH/files.txt" 2>&1
cp ~/Library/Logs/DiagnosticReports/*ackline* "$SH/" 2>/dev/null || true
cp ~/Library/Logs/DiagnosticReports/ios* "$SH/" 2>/dev/null || true
ls -la "$SH"
