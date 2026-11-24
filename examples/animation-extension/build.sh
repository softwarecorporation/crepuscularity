#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
out="$root/target/animation-extension"
mkdir -p "$out/examples/animation-browser" "$out/crates/crepuscularity-wasm/npm/browser" "$out/docs"
cp "$root/examples/animation-extension/manifest.json" "$root/examples/animation-extension/background.js" "$out/"
cp "$root/examples/animation-browser/"{index.html,motion.js} "$out/examples/animation-browser/"
cp "$root/crates/crepuscularity-wasm/npm/browser.js" "$out/crates/crepuscularity-wasm/npm/"
cp "$root/crates/crepuscularity-wasm/npm/browser/"{crepuscularity_wasm.js,crepuscularity_wasm_bg.wasm} "$out/crates/crepuscularity-wasm/npm/browser/"
cp "$root/docs/animation.md" "$out/docs/"
printf 'Load unpacked extension: %s\n' "$out"
