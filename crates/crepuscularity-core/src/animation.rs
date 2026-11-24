//! Bind portable animation values to the context consumed by every Rust renderer.
pub use crepuscularity_animation::{Easing, Frame, MotionValue, Transition};

use crate::{TemplateContext, TemplateValue};
use std::collections::BTreeMap;

/// Host-owned transitions. Advance before rendering; no global timer or threads.
#[derive(Debug, Default)]
pub struct AnimatedValues(BTreeMap<String, MotionValue>);

impl AnimatedValues {
    /// Animate an existing numeric binding. Missing/non-numeric values are errors.
    pub fn animate(
        &mut self,
        ctx: &mut TemplateContext,
        key: &str,
        target: f64,
        transition: Transition,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Result<Frame, &'static str> {
        let initial = match ctx.get(key) {
            Some(TemplateValue::Int(value)) => *value as f64,
            Some(TemplateValue::Float(value)) => *value,
            _ => return Err("animation binding must contain a number"),
        };
        // Validate before inserting, so errors do not leave a pending transition.
        let mut value = self
            .0
            .get(key)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| MotionValue::new(initial))?;
        let frame = value.animate_to(target, transition, now_ms, reduced_motion)?;
        ctx.set(key, frame.value);
        if frame.active {
            self.0.insert(key.to_owned(), value);
        } else {
            self.0.remove(key);
        }
        Ok(frame)
    }

    /// Publish a frame to the context. Returns whether another frame is needed.
    pub fn tick(&mut self, ctx: &mut TemplateContext, now_ms: u64, reduced_motion: bool) -> bool {
        self.0.retain(|key, value| {
            let frame = value.tick(now_ms, reduced_motion);
            ctx.set(key.as_str(), frame.value);
            frame.active
        });
        !self.0.is_empty()
    }

    /// Stop a binding at its last published value before setting it directly.
    pub fn cancel(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_bindings_cancel_and_errors_leave_context_unchanged() {
        let mut ctx = TemplateContext::new();
        ctx.set("width", 12).set("offset", -20);
        let mut motion = AnimatedValues::default();
        let transition = Transition {
            duration_ms: 200,
            easing: Easing::Linear,
        };
        motion
            .animate(&mut ctx, "width", 92.0, transition, 0, false)
            .unwrap();
        motion
            .animate(&mut ctx, "offset", 20.0, transition, 0, false)
            .unwrap();
        assert!(motion.tick(&mut ctx, 50, false));
        assert_eq!(ctx.get_str("width"), "32");
        assert_eq!(ctx.get_str("offset"), "-10");
        assert!(motion
            .animate(&mut ctx, "width", f64::NAN, transition, 50, false)
            .is_err());
        assert!(motion
            .animate(&mut ctx, "missing", 1.0, transition, 50, false)
            .is_err());
        motion.cancel("width");
        ctx.set("width", 7);
        assert!(!motion.tick(&mut ctx, 100, true));
        assert_eq!(ctx.get_str("width"), "7");
        assert_eq!(ctx.get_str("offset"), "20");
        assert!(ctx.get("missing").is_none());
    }
}
