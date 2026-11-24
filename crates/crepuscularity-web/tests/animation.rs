use crepuscularity_core::{
    animation::{AnimatedValues, Easing, Transition},
    TemplateContext,
};
use crepuscularity_web::render_template_to_html;

#[test]
fn sampled_motion_reaches_html_and_ssr_without_a_browser_clock() {
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
        let source = "div #panel class=\"w-[{width}px]\"\n  \"{width}\"";
        let html = render_template_to_html(source, &ctx).unwrap();
        assert!(html.contains(&format!("w-[{width}px]")), "{html}");
        assert!(html.contains(&format!(">{width}</div>")), "{html}");
        #[cfg(feature = "ssr")]
        assert!(
            crepuscularity_web::render_template_to_html_with_ssr(source, &ctx, false)
                .unwrap()
                .contains(&format!("w-[{width}px]"))
        );
    }
}
