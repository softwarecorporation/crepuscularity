import { loadMotionValue } from '../../crates/crepuscularity-wasm/npm/browser.js';

const toggle = document.querySelector('#toggle');
const panel = document.querySelector('#panel');
const details = document.querySelector('#details');
const reduce = document.querySelector('#reduce');
const policy = document.querySelector('#policy');
const system = matchMedia('(prefers-reduced-motion: reduce)');
let motion;
let open = false;
let pending;
const reduced = () => reduce.checked || system.matches;

function paint(value) {
  panel.style.height = `${value}px`;
  panel.dataset.active = String(motion.active);
  panel.dataset.value = String(value);
}

function frame(now) {
  pending = undefined;
  paint(motion.tick(now, reduced()));
  if (motion.active) pending = requestAnimationFrame(frame);
}

function policyChanged() {
  policy.textContent = reduced() ? 'Reduced motion: changes are immediate.' : 'Motion enabled: 240 ms, ease-in-out.';
  if (motion) {
    if (pending !== undefined) cancelAnimationFrame(pending);
    frame(performance.now());
  }
}

toggle.addEventListener('click', event => {
  open = !open;
  toggle.setAttribute('aria-expanded', String(open));
  panel.inert = !open;
  document.querySelector('#indicator').textContent = open ? '−' : '+';
  if (pending !== undefined) cancelAnimationFrame(pending);
  pending = undefined;
  // Keyboard activation is immediate; pointer activation demonstrates the transition.
  paint(motion.animateTo(open ? details.offsetHeight : 0, 240, 'ease-in-out', performance.now(), reduced() || event.detail === 0));
  if (motion.active) pending = requestAnimationFrame(frame);
});
reduce.addEventListener('change', policyChanged);
system.addEventListener('change', policyChanged);
new ResizeObserver(() => {
  if (!motion || !open) return;
  paint(motion.animateTo(details.offsetHeight, 240, 'ease-in-out', performance.now(), reduced()));
  if (motion.active && pending === undefined) pending = requestAnimationFrame(frame);
}).observe(details);

try {
  const MotionValue = await loadMotionValue();
  motion = new MotionValue(0);
  paint(0);
  toggle.disabled = false;
  policyChanged();
} catch (error) {
  policy.textContent = 'Engine unavailable. Build the WASM bindings as described in docs/animation.md.';
  console.error(error);
}
addEventListener('pagehide', () => {
  if (pending !== undefined) cancelAnimationFrame(pending);
  pending = undefined;
  if (motion) paint(motion.tick(performance.now(), true));
});
