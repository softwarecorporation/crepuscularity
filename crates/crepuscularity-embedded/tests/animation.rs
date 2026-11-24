#![cfg(feature = "std")]
use crepuscularity_core::{
    animation::{AnimatedValues, Easing, Transition},
    TemplateContext,
};
use crepuscularity_embedded::{
    render_template_to_framebuffer, Rgb565Buffer, ScreenSize, DEFAULT_BG,
};

#[test]
fn sampled_motion_changes_layout_and_real_framebuffer_pixels() {
    let mut ctx = TemplateContext::new();
    ctx.set("width", 12);
    let mut motion = AnimatedValues::default();
    motion
        .animate(
            &mut ctx,
            "width",
            92.0,
            Transition {
                duration_ms: 200,
                easing: Easing::Linear,
            },
            0,
            false,
        )
        .unwrap();
    let size = ScreenSize::new(100, 16);
    for (time, reduced, width, active) in [
        (0, false, 12, true),
        (50, false, 32, true),
        (75, true, 92, false),
        (100, false, 92, false),
    ] {
        assert_eq!(motion.tick(&mut ctx, time, reduced), active);
        let mut fb = Rgb565Buffer::new(size, DEFAULT_BG);
        let doc = render_template_to_framebuffer(
            "div w-full h-full bg-black\n  div #panel class=\"w-[{width}px] h-[8px] bg-red-500\"",
            &ctx,
            size,
            &mut fb,
        )
        .unwrap();
        assert_eq!(doc.node_by_id("panel").unwrap().bounds.w, width);
        assert_ne!(fb.pixels()[width as usize - 1], fb.pixels()[width as usize]);
    }
}
