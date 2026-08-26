#!/usr/bin/env bash
set -euo pipefail
# Interactive screenshot helper — replicates what Claude did:
# launches the app, then prompts for 4 window captures via screencapture -w.
# Requires Screen Recording permission for your terminal (System Settings → Privacy → Screen Recording).

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Building…"
cargo build

echo "Launching CrittoUtil…"
cargo run &
APP_PID=$!
trap 'kill $APP_PID 2>/dev/null || true' EXIT
sleep 3

capture() {
  local file="$1" hint="$2"
  echo ""
  echo "→ $hint"
  echo "  Clicca la finestra di CrittoUtil quando il mirino compare…"
  # -w = window selection, -x = no sound
  screencapture -w -x "$file"
  echo "  ✓ salvato $file ($(du -h "$file" | cut -f1))"
}

mkdir -p screenshots
capture "screenshots/02-home.png"      "1/4 — porta l'app su Home"
capture "screenshots/01-converter.png" "2/4 — vai su Converter"
capture "screenshots/03-encrypter.png" "3/4 — vai su Encrypter"
capture "screenshots/04-keygen.png"    "4/4 — vai su Key Generator"

kill $APP_PID 2>/dev/null || true
wait $APP_PID 2>/dev/null || true
trap - EXIT

echo ""
echo "Fatto. Anteprima:"
ls -lh screenshots/*.png
echo ""
echo "Per pushare:"
echo "  git add screenshots/*.png && git commit -m 'docs: refresh screenshots (edge-to-edge sidebar)' && git push"
