//! Enter/click animates; Space toggles immediately; R changes reduced motion; Q quits.
//! `--reduce-motion` starts with the explicit terminal accessibility policy enabled.

use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use crepuscularity_core::animation::{AnimatedValues, Easing, Transition};
use crepuscularity_tui::Template;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

struct Disclosure {
    ui: Template,
    motion: AnimatedValues,
    expanded: bool,
    reduced: bool,
    active: bool,
    frames: u64,
}

impl Disclosure {
    fn new(reduced: bool) -> Self {
        let mut ui = Template::from_source(
            "div flex-col\n  div h-[2]\n    span text-cyan-400 \"Shared motion / terminal\"\n  div h-[2]\n    span \"Enter/click: animate | Space: immediate | R: reduce motion | Q: quit\"\n  if {height >= 1}\n    div class=\"h-[{height.round}] bg-gray-800\"\n      span \"One engine, terminal frames. Reverse while moving.\"\n  div h-[2]\n    span \"{status}\"",
        );
        ui.set("height", 0);
        Self {
            ui,
            motion: AnimatedValues::default(),
            expanded: false,
            reduced,
            active: false,
            frames: 0,
        }
    }

    fn toggle(&mut self, now: u64, immediate: bool) {
        self.expanded = !self.expanded;
        self.motion
            .animate(
                self.ui.context_mut(),
                "height",
                if self.expanded { 8.0 } else { 0.0 },
                Transition {
                    duration_ms: 240,
                    easing: Easing::EaseInOut,
                },
                now,
                self.reduced || immediate,
            )
            .expect("numeric example height");
        self.tick(now);
    }

    fn tick(&mut self, now: u64) {
        self.active = self.motion.tick(self.ui.context_mut(), now, self.reduced);
        self.frames += 1;
        let height = self.ui.context_mut().get_str("height");
        self.ui.set(
            "status",
            format!(
                "Frames: {} | {} | Reduced motion: {} | Height: {}",
                self.frames,
                if self.active { "Active" } else { "Idle" },
                self.reduced,
                height
            ),
        );
    }

    fn frame_timeout(&self) -> Option<Duration> {
        self.active.then_some(Duration::from_millis(16))
    }
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    reduced: bool,
) -> Result<(), Box<dyn Error>> {
    let epoch = Instant::now();
    let mut host = Disclosure::new(reduced);
    host.tick(0);
    loop {
        let mut draw = Ok(());
        terminal.draw(|frame| {
            draw = host.ui.draw_full(frame);
        })?;
        draw?;
        // No timer or polling runs at idle: read blocks until the next input/resize.
        let deadline = host.frame_timeout().map(|delay| Instant::now() + delay);
        loop {
            let event = match deadline {
                Some(deadline) if deadline <= Instant::now() => None,
                Some(deadline)
                    if !event::poll(deadline.saturating_duration_since(Instant::now()))? =>
                {
                    None
                }
                _ => Some(event::read()?),
            };
            let now = epoch.elapsed().as_millis() as u64;
            match event {
                None => host.tick(now),
                Some(Event::Key(key)) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Enter => host.toggle(now, false),
                    KeyCode::Char(' ') => host.toggle(now, true),
                    KeyCode::Char('r') => {
                        host.reduced = !host.reduced;
                        host.tick(now);
                    }
                    _ => continue,
                },
                Some(Event::Mouse(mouse))
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    host.toggle(now, false);
                }
                Some(Event::Resize(_, _)) => host.tick(now),
                _ => continue,
            }
            break;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let reduced = std::env::args().skip(1).any(|arg| arg == "--reduce-motion");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let setup = execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture);
    if let Err(error) = setup {
        let _ = execute!(stdout, event::DisableMouseCapture);
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();
        return Err(error.into());
    }
    let result = Terminal::new(CrosstermBackend::new(stdout))
        .map_err(Into::into)
        .and_then(|mut terminal| run(&mut terminal, reduced));
    let restore = execute!(
        io::stdout(),
        event::DisableMouseCapture,
        LeaveAlternateScreen
    );
    let raw = disable_raw_mode();
    result?;
    restore?;
    raw?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn interrupted_cells_policy_changes_and_idle_scheduling() {
        let mut host = Disclosure::new(false);
        let mut terminal = Terminal::new(TestBackend::new(90, 20)).unwrap();
        host.toggle(0, false);
        host.tick(90);
        assert!(host.frame_timeout().is_some());
        terminal
            .draw(|frame| host.ui.draw_full(frame).unwrap())
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(0, 6)].symbol(), "F");
        let intermediate = host.ui.context_mut().get_str("height");
        host.toggle(90, false);
        assert_eq!(host.ui.context_mut().get_str("height"), intermediate);
        host.tick(150);
        let reversed = host
            .ui
            .context_mut()
            .get_str("height")
            .parse::<f64>()
            .unwrap();
        assert!(reversed > 0.0 && reversed < intermediate.parse::<f64>().unwrap());
        host.reduced = true;
        host.tick(151);
        assert_eq!(host.ui.context_mut().get_str("height"), "0");
        assert_eq!(host.frame_timeout(), None);
        host.toggle(152, false);
        assert_eq!(host.ui.context_mut().get_str("height"), "8");
        assert_eq!(host.frame_timeout(), None);
        terminal
            .draw(|frame| host.ui.draw_full(frame).unwrap())
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 4)].symbol(), "O");
        assert_eq!(buffer[(0, 12)].symbol(), "F");
        host.reduced = false;
        host.tick(200);
        assert_eq!(host.frame_timeout(), None);
        host.toggle(201, true);
        assert_eq!(host.ui.context_mut().get_str("height"), "0");
        assert_eq!(host.frame_timeout(), None);
        terminal
            .draw(|frame| host.ui.draw_full(frame).unwrap())
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(0, 4)].symbol(), "F");
    }
}
