# Animation engine

`crepuscularity-animation` provides interruptible numeric transitions with no dependencies, allocator, system clock, or `std` requirement. The same engine builds for desktop, WASM, mobile Rust libraries, and firmware. It does not create an event loop or change the View IR schema.

## Runtime contract

- `MotionValue::new(value)` rejects non-finite values.
- `animate_to(target, transition, now_ms, reduced_motion)` starts or retargets a transition. Retargeting samples the old transition first, so reversal does not jump to an endpoint.
- Repeating the same target and transition does not restart an active animation.
- `tick(now_ms, reduced_motion)` returns `Frame { value, active }`. Hosts render every returned frame, including the final inactive frame, and request another frame only while `active` is true.
- Timestamps are monotonic milliseconds with one time origin per value. Regressing timestamps clamp to the last timestamp. Long gaps finish at the exact target.
- Reduced motion and zero duration settle immediately at the target. Disabling reduced motion does not replay a completed animation.
- `cancel()` freezes the last sampled value.

`Transition` contains `duration_ms: u64` and an `Easing`: `Linear`, `EaseInOut`, `Quadratic`, or `EaseOut`. The curves retain the corresponding existing GPUI behavior. Springs, repeat cycles, keyframe sequences, and color interpolation are not part of this API.

## Rust template bindings

`crepuscularity_core::animation::AnimatedValues` publishes sampled numbers into an existing `TemplateContext`. All Rust renderers consume that context without a separate animation implementation.

```rust
use crepuscularity_core::{
	animation::{AnimatedValues, Easing, Transition},
	TemplateContext,
};

let mut ctx = TemplateContext::new();
ctx.set("width", 12);
let mut motion = AnimatedValues::default();
motion.animate(
	&mut ctx, "width", 92.0,
	Transition { duration_ms: 200, easing: Easing::EaseInOut },
	0, false,
).unwrap();
let request_another_frame = motion.tick(&mut ctx, 50, false);
// Render with ctx now, even when request_another_frame is false.
```

For GPUI, native IR, and RGB565 templates, `div class="w-[{width}px]"` consumes this binding. Terminal dimensions use cells: `div class="w-[{width}]"`. LVGL supports a numeric property binding: `div width={width}`. On the web, interpolated class names still need matching generated CSS. For continuous dimensions, bind the sampled number to an inline style or a DOM property rather than expecting a CSS generator to discover every frame.

`AnimatedValues::animate` requires an existing numeric binding. `cancel(key)` stops that binding before a direct context update. `clear()` stops all bindings. The caller owns both the context and the animation collection. Keep them together rather than reusing a collection with a different context.

## Platform integration

| Backend | Engine access | Host responsibility |
| --- | --- | --- |
| GPUI desktop, including the Lite host | `core::animation`, also re-exported by `crepuscularity_gpui::animation` | Tick before render, then request a GPUI frame while active. |
| Web, Tauri webview, browser extensions | WASM `MotionValue` | Use `requestAnimationFrame` and `matchMedia('(prefers-reduced-motion: reduce)')`. A service worker has no visual frame loop. |
| Static HTML and SSR | `AnimatedValues` and the web renderer | Emit a chosen snapshot. SSR does not run a client clock. |
| SwiftUI, Compose, Flutter View IR consumers | C ABI sessions or Rust `AnimatedValues`, followed by new IR | Deliver each sampled IR to the shell. A fixed JSON fixture cannot animate itself. |
| Experimental ZPUI Zig adapter | C ABI sessions through generated Zig imports, followed by new IR | Replace the retained document through the host's frame lifecycle, then notify the ZPUI view. |
| Ratatui terminal | `AnimatedValues` and `Template::context_mut()` | Tick on the terminal event loop, then draw. Choose a user preference for reduced motion. |
| RGB565 with `std` | `AnimatedValues` and the embedded context | Tick before layout and framebuffer paint. Flush the frame to the panel. |
| RGB565 without `std` | `crepuscularity_embedded::animation::MotionValue` | Supply firmware ticks. Apply sampled dimensions to a compiled document before layout and paint. |
| LVGL XML | `AnimatedValues` and numeric property bindings | Consume the sampled XML or apply values to live LVGL objects. XML generation alone is not a firmware scheduler. |

Native hosts supply their current accessibility preference on every frame: SwiftUI's `accessibilityReduceMotion`, Android's disabled animation policy, or Flutter's `MediaQuery.disableAnimationsOf(context)`. The Rust engine does not query operating-system settings. Re-read the preference when it changes, not just when the animation starts.

Existing GPUI `animate:*` directives retain their mount-animation behavior and identity. Setting `crepus_reduced_motion` to `true` in their template context bypasses animation wrappers and renders their final state, including repeating directives. Existing GPUI width, height, and opacity helper signatures remain compatible; applications needing reduced motion use the caller-clocked API or render the final style directly.

The new portable API animates values, not arbitrary AST directives. Existing web `data-animate-*` metadata does not become an automatic animation, and native shell fixtures and the pure-Dart parser do not acquire frame loops. Backend property support still applies: terminal cells cannot represent fractional opacity, and the embedded painter does not gain alpha blending.

## C ABI sessions

`crepus_session_animate_json` accepts this JSON and returns `0` on success or `-1` on error:

```json
{"key":"width","target":92,"durationMs":200,"easing":"ease-in-out","nowMs":0,"reducedMotion":false}
```

`crepus_session_tick_animations(session, now_ms, reduced_motion)` returns `1` while active, `0` when settled, or `-1` on error. `reduced_motion` must be `0` or `1`. Render IR after each successful tick, including the final `0`. Context replacement cancels all animation. A context patch or bind event cancels animation for each key it overwrites. Other keys keep their transitions. Session re-entry and ownership rules remain unchanged.

## Browser example

`examples/animation-browser/index.html` uses the actual WASM engine for a disclosure panel. Pointer clicks animate and can reverse mid-transition. Keyboard activation is immediate. The example combines an explicit reduce-motion control with the system preference, handles live preference changes, and stops requesting frames when settled.

The example imports the checked-in npm browser entry. Rebuild its bindings from the repository root with a `wasm-bindgen` CLI matching the version in `Cargo.lock`:

```sh
bash crates/crepuscularity-wasm/build.sh
```

Serve the repository root, then open `/examples/animation-browser/`. In an Amp orb, a supervised preview can use:

```sh
amp orb service start animation-preview --command 'python3 -m http.server "$PORT" --bind 0.0.0.0' --port 4100 --portal
```

The command returns the public portal URL. The WASM build regenerates the existing checked-in npm and legacy bindings.

## Native GPUI example

Run `cargo run --locked -p crepuscularity-gpui --example animated_disclosure` on macOS. On Linux, also select a platform backend, for example `--features x11`.

The example samples `AnimatedValues` during native render callbacks and calls `Window::request_animation_frame` only while a transition is active. The disclosure's height is a real template binding, not a second animation implementation. Pointer clicks can reverse a transition. Space toggles immediately, and R or the app control changes reduced motion, including during an active transition. The frame counter stops changing when idle.

This example uses GPUI's application-level `reduce_motion` policy. It does not query operating-system accessibility settings. A production host must keep that policy in sync with the user's current OS or application preference. Testing this Mac example does not establish Windows or Linux presentation coverage.

## Verification commands

```sh
cargo test --locked -p crepuscularity-animation -p crepuscularity-core --lib
cargo test --locked -p crepuscularity-web -p crepuscularity-native -p crepuscularity-tui -p crepuscularity-embedded -p crepuscularity-lvgl --test animation
cargo test --locked -p crepuscularity-embedded --no-default-features --test animation_no_std
cargo test --locked -p crepuscularity-abi --test session_contract
cargo test --locked -p crepuscularity-runtime --lib motion
```

`crates/crepuscularity-wasm/build.sh` also runs `npm/animation-smoke.mjs` against generated WASM bindings. Tests assert intermediate values, interruption, endpoint completion, reduced-motion changes, invalid inputs, and idle state. Renderer checks inspect HTML, IR dimensions, terminal cells, RGB565 pixels, and LVGL XML. Those checks do not prove physical panel timing, mobile accessibility integration, an installed extension, or native GPU presentation.
