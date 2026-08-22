#!/bin/sh
# Render a xilem example headless to screenshots/<name>.png.
# Usage: tools/screenshot.sh <example> [more examples...]
set -e
cd "$(dirname "$0")/.."
for ex in "$@"; do
  XIX_SCREENSHOT="screenshots/$ex.png" cargo run -q -p xilem --features screenshot --example "$ex"
  echo "screenshots/$ex.png"
done
