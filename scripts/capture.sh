#!/usr/bin/env bash
set -euo pipefail
# Auto screenshot helper — like Claude: launches the app, auto-clicks sidebar items, captures windows non-interactively.
# Requires: Screen Recording permission for your terminal + cliclick (brew install cliclick) for auto-clicks.
# Falls back to manual screencapture -w if cliclick is missing.
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Building…"
cargo build

echo "Launching CrittoUtil…"
cargo run &
APP_PID=$!
trap 'kill $APP_PID 2>/dev/null || true' EXIT
sleep 4

# Get window bounds via AppleScript
get_bounds() {
  osascript -e 'tell application "System Events" to get {position, size} of window 1 of process "gpui-crittoutil"' 2>/dev/null || echo ""
}

if ! command -v cliclick &>/dev/null; then
  echo "cliclick non trovato (brew install cliclick) — fallback manuale con screencapture -w"
  for f in "screenshots/02-home.png:Home" "screenshots/01-converter.png:Converter" "screenshots/03-encrypter.png:Encrypter" "screenshots/04-keygen.png:Key Generator"; do
    file="${f%%:*}"; hint="${f##*:}"
    echo "→ $hint — clicca la finestra quando compare il mirino"
    screencapture -w -x "$file"
  done
else
  # Window top-left + sidebar nav items (~80px from left, ~140px + n*36px from top of window)
  BOUNDS=$(get_bounds)
  echo "Window bounds: $BOUNDS"
  # Parse: {{x, y}, {w, h}}
  WX=$(echo "$BOUNDS" | sed -E 's/.*\{\{([0-9]+),.*/\1/')
  WY=$(echo "$BOUNDS" | sed -E 's/.*\{\{[0-9]+, ([0-9]+)\}.*/\1/')
  if [ -z "$WX" ] || [ "$WX" = "$BOUNDS" ]; then WX=100; WY=100; fi

  click_nav() {
    local idx=$1
    local cx=$((WX + 80))
    local cy=$((WY + 140 + idx * 36))
    cliclick c:$cx,$cy
    sleep 0.6
  }

  capture_win() {
    local file="$1"
    sleep 0.4
    # -l <windowID> captures specific window; get ID via Quartz
    WID=$(python3 -c "import Quartz; ws=Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly, Quartz.kCGNullWindowID); print(next((w['kCGWindowNumber'] for w in ws if w.get('kCGWindowOwnerName')=='gpui-crittoutil'), ''))" 2>/dev/null || echo "")
    if [ -n "$WID" ]; then
      screencapture -l"$WID" -x "$file" 2>/dev/null || screencapture -w -x "$file"
    else
      screencapture -w -x "$file"
    fi
    echo "  ✓ $file"
  }

  mkdir -p screenshots
  # Home is idx 0 — already there, but click to ensure
  click_nav 0; capture_win "screenshots/02-home.png"
  click_nav 1; capture_win "screenshots/01-converter.png"
  click_nav 3; capture_win "screenshots/03-encrypter.png"
  click_nav 2; capture_win "screenshots/04-keygen.png"
fi

kill $APP_PID 2>/dev/null || true
wait $APP_PID 2>/dev/null || true
trap - EXIT
echo "Fatto:"; ls -lh screenshots/*.png
