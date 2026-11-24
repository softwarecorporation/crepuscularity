//! Run with `cargo run -p crepuscularity-gpui --example animated_disclosure`.
//! Space toggles immediately. R changes the app's reduced-motion policy.
//! The app policy is explicit here; GPUI does not query the OS preference.

use std::time::Instant;

use crepuscularity_core::{ast::Node, parse_template, TemplateContext};
use crepuscularity_gpui::{
    animation::{AnimatedValues, Easing, Transition},
    gpui::{actions, bounds, point, size, FocusHandle, KeyBinding, WindowBounds},
    prelude::*,
};
use crepuscularity_runtime::renderer::render_nodes;

actions!(motion_demo, [TogglePanel, ToggleReducedMotion]);

struct Disclosure {
    epoch: Instant,
    context: TemplateContext,
    motion: AnimatedValues,
    nodes: Vec<Node>,
    expanded: bool,
    focus: FocusHandle,
    frames: u64,
}

impl Disclosure {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut context = TemplateContext::new();
        context.set("height", 0);
        Self {
            epoch: Instant::now(),
            context,
            motion: AnimatedValues::default(),
            nodes: parse_template(
                r#"div class="h-[{height}px] w-full overflow-hidden bg-zinc-100"
  div p-4 text-zinc-900
    div mb-2
      "One engine, native frames."
    div
      "Reverse mid-transition or enable reduced motion.""#,
            )
            .expect("valid example template"),
            expanded: false,
            focus: cx.focus_handle(),
            frames: 0,
        }
    }

    fn toggle(&mut self, immediate: bool, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        self.motion
            .animate(
                &mut self.context,
                "height",
                if self.expanded { 112.0 } else { 0.0 },
                Transition {
                    duration_ms: 240,
                    easing: Easing::EaseInOut,
                },
                self.epoch.elapsed().as_millis() as u64,
                immediate || cx.reduce_motion(),
            )
            .expect("existing numeric height binding");
        cx.notify();
    }
}

impl Render for Disclosure {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.motion.tick(
            &mut self.context,
            self.epoch.elapsed().as_millis() as u64,
            cx.reduce_motion(),
        );
        if active {
            window.request_animation_frame();
        }
        self.frames += 1;
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .bg(rgb(0xfaf9f6))
            .text_color(rgb(0x18181b))
            .track_focus(&self.focus)
            .on_action(cx.listener(|view, _: &TogglePanel, _, cx| view.toggle(true, cx)))
            .on_action(cx.listener(|_, _: &ToggleReducedMotion, _, cx| {
                let reduced = !cx.reduce_motion();
                cx.set_reduce_motion(reduced);
            }))
            .child(div().text_2xl().child("Native disclosure"))
            .child("Space toggles immediately. R changes reduced motion.")
            .child(
                div()
                    .id("panel-toggle")
                    .p_4()
                    .border_1()
                    .border_color(rgb(0xa1a1aa))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xe4e4e7)))
                    .on_click(cx.listener(|view, _, _, cx| view.toggle(false, cx)))
                    .child(if self.expanded {
                        "Build targets  -"
                    } else {
                        "Build targets  +"
                    }),
            )
            .child(render_nodes(&self.nodes, &self.context))
            .child(
                div()
                    .id("reduced-motion")
                    .p_3()
                    .border_1()
                    .border_color(rgb(0xa1a1aa))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xe4e4e7)))
                    .on_click(cx.listener(|_, _, _, cx| {
                        let reduced = !cx.reduce_motion();
                        cx.set_reduce_motion(reduced);
                    }))
                    .child(if cx.reduce_motion() {
                        "App reduced motion: on"
                    } else {
                        "App reduced motion: off"
                    }),
            )
            .child(format!(
                "Rendered frames: {}. {}",
                self.frames,
                if active { "Animating" } else { "Idle" }
            ))
    }
}

fn main() {
    application().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("space", TogglePanel, None),
            KeyBinding::new("r", ToggleReducedMotion, None),
        ]);
        cx.open_window(
            gpui_window_options(
                "crepus.motion",
                "Crepuscularity motion",
                Some(WindowBounds::Windowed(bounds(
                    point(px(120.0), px(80.0)),
                    size(px(680.0), px(500.0)),
                ))),
                None,
            ),
            |window, cx| {
                let view = cx.new(Disclosure::new);
                let focus = view.read(cx).focus.clone();
                window.focus(&focus, cx);
                view
            },
        )
        .expect("open native window");
        cx.activate(true);
    });
}
