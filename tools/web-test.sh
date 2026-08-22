#!/bin/sh
# Serve the repo and screenshot tools/web/index.html with headless Chrome.
# Usage: tools/web-test.sh [out.png] [window-size WxH]
set -e
cd "$(dirname "$0")/.."
out="${1:-screenshots/web-calc.png}"; size="${2:-800,640}"
port=8787
python3 -m http.server "$port" --bind 127.0.0.1 >/Users/eli/Temp/xix-web-server.log 2>&1 &
pid=$!
trap 'kill $pid 2>/dev/null' EXIT
sleep 1
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
"$chrome" --headless=new --disable-gpu --hide-scrollbars --window-size="$size" \
  --virtual-time-budget=4000 --run-all-compositor-stages-before-draw \
  --screenshot="$out" "http://127.0.0.1:$port/tools/web/index.html" 2>/Users/eli/Temp/xix-chrome.log
echo "$out"
