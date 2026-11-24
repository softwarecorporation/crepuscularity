use crepuscularity_embedded::{
    animation::{Easing, MotionValue, Transition},
    compiled::{CElement, CNode, CompiledTemplate},
    layout::layout_document,
    paint::paint_document,
    Color, Rgb565Buffer, ScreenSize, SizeHint,
};

#[test]
fn firmware_clock_drives_compiled_layout_without_the_runtime_parser() {
    let template = CompiledTemplate {
        nodes: &[CNode::Element(CElement {
            tag: "div",
            id: None,
            classes: &["w-full", "h-full", "bg-black"],
            conditional_classes: &[],
            on_click: None,
            children: &[CNode::Element(CElement {
                tag: "div",
                id: Some("panel"),
                classes: &["h-[8px]", "bg-red-500"],
                conditional_classes: &[],
                on_click: None,
                children: &[],
            })],
        })],
    };
    let screen = ScreenSize::new(100, 16);
    let mut motion = MotionValue::new(12.0).unwrap();
    motion
        .animate_to(
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
    ] {
        let frame = motion.tick(time, reduced);
        assert_eq!(frame.value, f64::from(width));
        assert_eq!(frame.active, active);
        let mut doc = template.document(&(), screen);
        doc.root[0].children[0].style.width = SizeHint::Fixed(width);
        layout_document(&mut doc);
        let mut fb = Rgb565Buffer::new(screen, Color::rgb(0, 0, 0));
        paint_document(&mut fb, &doc);
        assert_eq!(doc.node_by_id("panel").unwrap().bounds.w, width);
        assert_ne!(fb.pixels()[width as usize - 1], fb.pixels()[width as usize]);
    }
}
