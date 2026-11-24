use crepuscularity_core::{
    animation::{AnimatedValues, Easing, Transition},
    TemplateContext,
};
use crepuscularity_native::render_template_to_ir;

#[test]
fn sampled_motion_reaches_native_ir_layout() {
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
        (0, false, 12.0, true),
        (50, false, 32.0, true),
        (75, true, 92.0, false),
        (100, false, 92.0, false),
    ] {
        assert_eq!(motion.tick(&mut ctx, time, reduced), active);
        let ir = render_template_to_ir("div #panel class=\"w-[{width}px]\"", &ctx).unwrap();
        let json = serde_json::to_value(ir).unwrap();
        assert_eq!(json["root"][0]["style"]["width"], width);
    }
}
