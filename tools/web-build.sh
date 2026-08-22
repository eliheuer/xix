#!/bin/sh
# Build a xilem example for the browser: tools/web-build.sh <example>
# Produces dist/web/<example>/{<example>_web.js,<example>_web_bg.wasm}.
set -e
cd "$(dirname "$0")/.."
ex="$1"; [ -n "$ex" ] || { echo "usage: $0 <example>"; exit 2; }
cargo build --release --target wasm32-unknown-unknown -p xilem --features web --example "${ex}_web"
out="dist/web/$ex"; mkdir -p "$out"
"${WASM_BINDGEN:-wasm-bindgen}" --target web --no-typescript --out-dir "$out" \
  "target/wasm32-unknown-unknown/release/examples/${ex}_web.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz -o "$out/${ex}_web_bg.wasm" "$out/${ex}_web_bg.wasm"
fi
ls -la "$out"
