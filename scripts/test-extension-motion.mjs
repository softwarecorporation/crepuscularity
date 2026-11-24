// Run with Bun against an agent-browser session showing the installed options page.
import { execFileSync } from 'node:child_process';
import assert from 'node:assert/strict';

const session = process.argv[2];
if (!session) throw new Error('Usage: bun scripts/test-extension-motion.mjs <agent-browser-session>');
const browserUrl = execFileSync('agent-browser', ['--session', session, 'get', 'cdp-url'], { encoding: 'utf8' }).trim();
const targets = await fetch(new URL('/json/list', browserUrl.replace('ws:', 'http:'))).then(r => r.json());
const page = targets.find(t => t.type === 'page' && /^chrome-extension:\/\/[^/]+\/examples\/animation-browser\/index.html$/.test(t.url));
assert.ok(page, 'An installed extension options page must be open; HTTP previews do not qualify');
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let sequence = 0;
const requests = new Map();
const errors = [];
socket.onmessage = event => {
  const message = JSON.parse(event.data);
  if (message.method === 'Runtime.exceptionThrown') errors.push(message.params.exceptionDetails);
  if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') errors.push(message.params.entry);
  const request = requests.get(message.id);
  if (!request) return;
  requests.delete(message.id);
  clearTimeout(request.timeout);
  if (message.error) request.reject(new Error(JSON.stringify(message.error)));
  else request.resolve(message.result);
};
function cdp(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timeout = setTimeout(() => { requests.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 5000);
    requests.set(id, { resolve, reject, timeout });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const result = await cdp('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  assert.equal(result.exceptionDetails, undefined, JSON.stringify(result.exceptionDetails));
  return result.result.value;
}
async function until(predicate, label) {
  const deadline = Date.now() + 4000;
  do {
    if (await predicate()) return;
    await Bun.sleep(8);
  } while (Date.now() < deadline);
  throw new Error(`Timed out: ${label}`);
}
async function reload() {
  await evaluate('window.motionTestReloading = true');
  await cdp('Page.reload');
  await until(() => evaluate(`!window.motionTestReloading && document.readyState === 'complete'`), 'new document');
}
const state = () => evaluate(`({
  value: +panel.dataset.value, active: panel.dataset.active === 'true',
  target: details.offsetHeight, expanded: toggle.getAttribute('aria-expanded'),
  inert: panel.inert, frames: window.probe.frames, pending: window.probe.pending.size,
  reduced: matchMedia('(prefers-reduced-motion: reduce)').matches
})`);
async function instrument() {
  await until(() => evaluate(`!!document.querySelector('#toggle') && !toggle.disabled`), 'WASM ready');
  await evaluate(`(async () => {
    window.probe = {frames: 0, pending: new Set(), calls: [], inFrame: false};
    const {loadMotionValue} = await import(chrome.runtime.getURL('crates/crepuscularity-wasm/npm/browser.js'));
    const MotionValue = await loadMotionValue();
    for (const method of ['animateTo', 'tick']) {
      const original = MotionValue.prototype[method];
      MotionValue.prototype[method] = function(...args) {
        const call = {method, args, beforeActive: this.active, beforeValue: +panel.dataset.value, inFrame: probe.inFrame};
        call.value = original.apply(this, args);
        call.active = this.active;
        probe.calls.push(call);
        return call.value;
      };
    }
    const request = window.requestAnimationFrame.bind(window);
    const cancel = window.cancelAnimationFrame.bind(window);
    window.requestAnimationFrame = callback => {
      const id = request(now => {
        probe.pending.delete(id); probe.frames++; probe.inFrame = true;
        try { callback(now); } finally { probe.inFrame = false; }
      });
      probe.pending.add(id); return id;
    };
    window.cancelAnimationFrame = id => {probe.pending.delete(id); cancel(id);};
  })()`);
}
async function click(selector) {
  const box = await evaluate(`(() => {const b = document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect(); return {x:b.x+b.width/2,y:b.y+b.height/2};})()`);
  await cdp('Input.dispatchMouseEvent', { type: 'mouseMoved', ...box });
  await cdp('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...box });
  await cdp('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...box });
}
async function idle() {
  await until(async () => !(await state()).active, 'endpoint');
  const before = await state();
  assert.equal(before.pending, 0, 'No scheduled frame remains at idle');
  await Bun.sleep(350);
  const after = await state();
  assert.equal(after.frames, before.frames, 'No callbacks run at idle');
  assert.equal(after.pending, 0);
  return after;
}
const media = reduced => cdp('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: reduced ? 'reduce' : 'no-preference' }] });
const interior = async () => {
  const s = await state();
  return s.active && s.value > 5 && s.value < s.target * .7;
};
async function assertLivePolicy() {
  await until(() => evaluate(`probe.calls.some(c => c.method === 'tick' && c.args[1] && !c.inFrame)`), 'policy event tick');
  const calls = await evaluate('probe.calls');
  const animation = calls.findLast(c => c.method === 'animateTo');
  const tick = calls.find(c => c.method === 'tick' && c.args[1] && !c.inFrame);
  assert.equal(tick.beforeActive, true, 'Policy must arrive during active motion');
  assert.ok(tick.beforeValue > 0 && tick.beforeValue < animation.args[0], 'Policy must interrupt an interior frame');
  assert.ok(tick.args[0] < animation.args[3] + animation.args[1] - 30, 'Policy tick must precede natural completion');
  assert.equal(tick.active, false, 'The policy event itself must settle, not a later deadline');
  assert.equal(tick.value, animation.args[0]);
  console.log(`PASS policy event settled ${Math.round(animation.args[3] + animation.args[1] - tick.args[0])} ms before deadline`);
}
try {
  await cdp('Runtime.enable');
  await cdp('Log.enable');
  await media(false);
  await reload();
  await instrument();
  const manifest = await evaluate('chrome.runtime.getManifest()');
  assert.equal(manifest.manifest_version, 3);
  assert.equal(manifest.name, 'Crepuscularity motion host');
  assert.deepEqual(manifest.permissions ?? [], []);
  await idle();
  await click('#toggle');
  await until(interior, 'intermediate opening');
  await click('#toggle');
  const transitions = await evaluate(`probe.calls.filter(c => c.method === 'animateTo')`);
  const opening = transitions[0];
  const reversed = transitions.at(-1);
  assert.equal(reversed.beforeActive, true);
  assert.ok(reversed.beforeValue > 0 && reversed.beforeValue < opening.args[0]);
  const progress = (Math.floor(reversed.args[3]) - Math.floor(opening.args[3])) / opening.args[1];
  assert.ok(progress > 0 && progress < .7, 'Retarget must occur before the old transition completes');
  const fraction = progress < .5 ? 2 * progress ** 2 : 1 - (-2 * progress + 2) ** 2 / 2;
  const expected = opening.value + (opening.args[0] - opening.value) * fraction;
  assert.ok(Math.abs(reversed.value - expected) < 1e-7, 'Retarget must preserve the old curve at the actual engine timestamp');
  const reversal = await state();
  assert.ok(reversal.active && reversal.value > 0 && reversal.value < reversal.target);
  await until(async () => { const s = await state(); return s.active && s.value < reversal.value; }, 'reversed frame');
  assert.equal((await idle()).value, 0);
  console.log(`PASS installed MV3 WASM, trusted pointer, continuous in-flight retarget (${reversed.value.toFixed(2)}), final frame, idle`);

  await evaluate('probe.calls = []');
  await click('#toggle');
  await until(interior, 'active policy transition');
  await media(true);
  await assertLivePolicy();
  let settled = await idle();
  assert.equal(settled.value, settled.target);
  assert.equal(settled.expanded, 'true');
  assert.equal(settled.inert, false);
  await click('#toggle');
  settled = await state();
  assert.equal(settled.value, 0);
  assert.equal(settled.active, false);
  await idle();
  await media(false);
  await idle();
  assert.equal((await state()).value, 0, 'Disabling policy must not replay');
  console.log('PASS live browser reduced-motion policy, immediate pointer activation, no replay');

  await evaluate('probe.calls = []');
  await click('#toggle');
  await until(interior, 'explicit policy transition');
  await click('#reduce');
  await assertLivePolicy();
  settled = await idle();
  assert.equal(settled.value, settled.target);
  await click('#toggle');
  assert.equal((await state()).value, 0);
  assert.equal((await state()).active, false);
  await click('#reduce');
  await evaluate('toggle.focus()');
  await cdp('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' });
  await cdp('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 });
  settled = await state();
  assert.equal(settled.value, settled.target);
  assert.equal(settled.active, false);
  assert.equal(settled.pending, 0);
  await idle();
  console.log('PASS explicit live reduced motion, keyboard immediate activation and accessibility state');

  const wideHeight = settled.target;
  await cdp('Emulation.setDeviceMetricsOverride', { width: 280, height: 844, deviceScaleFactor: 2, mobile: false });
  await until(async () => { const s = await state(); return s.target > wideHeight && s.value === s.target && !s.active; }, 'resized endpoint');
  settled = await idle();
  assert.equal(settled.value, settled.target, 'Resize must retarget the open content');
  await cdp('Emulation.setDeviceMetricsOverride', { width: 1100, height: 800, deviceScaleFactor: 2, mobile: false });
  await until(async () => { const s = await state(); return s.target === wideHeight && s.value === s.target && !s.active; }, 'wide resized endpoint');
  settled = await idle();
  assert.equal(settled.value, wideHeight);
  console.log('PASS open content resize and idle after retargeting');

  await media(true);
  await reload();
  await instrument();
  await click('#toggle');
  settled = await state();
  assert.equal(settled.reduced, true);
  assert.equal(settled.value, settled.target);
  assert.equal(settled.active, false);
  await idle();
  await media(false);
  await idle();
  await click('#toggle');
  await until(async () => (await state()).active, 'teardown transition');
  await evaluate(`dispatchEvent(new PageTransitionEvent('pagehide'))`);
  settled = await idle();
  assert.equal(settled.value, 0);
  assert.equal(settled.inert, true);
  assert.deepEqual(errors, [], 'No CSP, WASM or runtime errors');
  console.log('PASS startup reduced motion, pagehide cancellation, no CSP/runtime errors');
} finally {
  await media(false);
  socket.close();
}
