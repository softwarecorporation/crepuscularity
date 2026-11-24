use crepuscularity_core::{
    animation::{AnimatedValues, Easing, Transition},
    TemplateContext,
};
use crepuscularity_lvgl::render_template_to_lvgl_xml;

#[test]
fn host_sampled_motion_reaches_lvgl_xml() {
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
    for (time, reduced, width, active) in [
        (0, false, 12, true),
        (50, false, 32, true),
        (75, true, 92, false),
        (100, false, 92, false),
    ] {
        assert_eq!(motion.tick(&mut ctx, time, reduced), active);
        let xml = render_template_to_lvgl_xml("div width={width}", &ctx).unwrap();
        assert!(xml.contains(&format!("width=\"{width}\"")), "{xml}");
    }
}
