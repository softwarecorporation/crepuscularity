use crepuscularity_core::animation::{AnimatedValues, Easing, Transition};
use crepuscularity_tui::Template;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn sampled_motion_invalidates_retained_terminal_cells() {
    let mut ui = Template::from_source(
        "div flex-row\n  div class=\"w-[{width}] h-[1] bg-red-500\"\n  span \"END\"",
    );
    ui.set("width", 12);
    let mut motion = AnimatedValues::default();
    motion
        .animate(
            ui.context_mut(),
            "width",
            52.0,
            Transition {
                duration_ms: 200,
                easing: Easing::Linear,
            },
            0,
            false,
        )
        .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 2)).unwrap();
    for (time, reduced, width, active) in [
        (0, false, 12, true),
        (50, false, 22, true),
        (75, true, 52, false),
        (100, false, 52, false),
    ] {
        assert_eq!(motion.tick(ui.context_mut(), time, reduced), active);
        terminal
            .draw(|frame| {
                ui.draw_full_if_changed(frame).unwrap();
            })
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(width, 0)].symbol(), "E");
        assert_eq!(terminal.backend().buffer()[(width + 2, 0)].symbol(), "D");
    }
}
