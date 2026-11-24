import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const require = createRequire(import.meta.url);
const { MotionValue } = require(process.argv[2] ? resolve(process.argv[2]) : "./wasm/crepuscularity_wasm.js");
const value = new MotionValue(12);
try {
  assert.equal(value.animateTo(92, 200, "linear", 0, false), 12);
  assert.equal(value.tick(50, false), 32);
  assert.equal(value.active, true);
  assert.equal(value.animateTo(-8, 200, "linear", 100, false), 52);
  assert.equal(value.tick(150, false), 37);
  assert.equal(value.tick(151, true), -8);
  assert.equal(value.active, false);
  assert.equal(value.tick(160, false), -8);
  assert.equal(value.animateTo(12, 0, "linear", 160, false), 12);
  for (const [target, duration, easing, now] of [[Infinity, 200, "linear", 160], [12, -1, "linear", 160], [12, 200, "typo", 160], [12, 200, "linear", NaN]]) {
    assert.throws(() => value.animateTo(target, duration, easing, now, false));
  }
  assert.equal(value.tick(170, false), 12);
  assert.equal(value.cancel(), 12);
  assert.equal(value.active, false);
} finally {
  value.free();
}
console.log("WASM animation: interpolation, interruption, reduced motion, idle and invalid-input checks passed");
