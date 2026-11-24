//! Portable animation values. No allocator, system clock, executor, or renderer is required.
//!
//! Feed monotonic milliseconds from the host's frame callback to [`MotionValue::tick`].
//! Render the returned value, and request another frame only while [`Frame::active`].
//! Pass the host's current reduced-motion preference on every tick and retarget.

#![no_std]

/// Easing curves shared by native, terminal, browser and firmware hosts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    #[default]
    Linear,
    EaseInOut,
    Quadratic,
    EaseOut,
}

impl Easing {
    pub fn sample(self, progress: f64) -> f64 {
        let t = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInOut if t < 0.5 => 2.0 * t * t,
            Self::EaseInOut => {
                let x = 2.0 - 2.0 * t;
                1.0 - x * x / 2.0
            }
            Self::Quadratic => t * t,
            Self::EaseOut => {
                let t = 1.0 - t;
                1.0 - t * t * t * t * t
            }
        }
    }
}

impl core::str::FromStr for Easing {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "linear" => Ok(Self::Linear),
            "ease-in-out" => Ok(Self::EaseInOut),
            "quadratic" => Ok(Self::Quadratic),
            "ease-out" => Ok(Self::EaseOut),
            _ => Err("unsupported easing"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub duration_ms: u64,
    pub easing: Easing,
}

/// A sampled value and whether the host should request another frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub value: f64,
    pub active: bool,
}

/// An interruptible scalar transition, suitable for layout, opacity, offsets or progress.
/// Retargeting starts at the currently presented value, not the previous endpoint.
#[derive(Clone, Debug)]
pub struct MotionValue {
    value: f64,
    from: f64,
    target: f64,
    started_ms: u64,
    last_ms: u64,
    transition: Transition,
    active: bool,
}

impl MotionValue {
    pub fn new(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("animation values must be finite");
        }
        Ok(Self {
            value,
            from: value,
            target: value,
            started_ms: 0,
            last_ms: 0,
            transition: Transition {
                duration_ms: 0,
                easing: Easing::Linear,
            },
            active: false,
        })
    }

    /// Identical target/configuration does not restart an in-flight transition.
    /// Invalid targets leave the existing transition untouched.
    pub fn animate_to(
        &mut self,
        target: f64,
        transition: Transition,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Result<Frame, &'static str> {
        if !target.is_finite() {
            return Err("animation values must be finite");
        }
        self.tick(now_ms, reduced_motion);
        if target != self.target || transition != self.transition {
            self.from = self.value;
            self.target = target;
            self.transition = transition;
            self.started_ms = self.last_ms;
            self.active = self.value != target;
        }
        Ok(self.tick(now_ms, reduced_motion))
    }

    /// Regressing clocks are clamped. Long gaps finish at the exact endpoint.
    /// Reduced motion settles permanently; switching it off does not replay motion.
    pub fn tick(&mut self, now_ms: u64, reduced_motion: bool) -> Frame {
        self.last_ms = self.last_ms.max(now_ms);
        if self.active {
            let elapsed = self.last_ms - self.started_ms;
            if reduced_motion || elapsed >= self.transition.duration_ms {
                self.value = self.target;
                self.active = false;
            } else {
                let progress = elapsed as f64 / self.transition.duration_ms as f64;
                let t = self.transition.easing.sample(progress);
                self.value = self.from * (1.0 - t) + self.target * t;
            }
        }
        Frame {
            value: self.value,
            active: self.active,
        }
    }

    /// Freeze at the last sampled value and stop requesting frames.
    pub fn cancel(&mut self) -> Frame {
        self.target = self.value;
        self.active = false;
        Frame {
            value: self.value,
            active: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LINEAR: Transition = Transition {
        duration_ms: 200,
        easing: Easing::Linear,
    };

    #[test]
    fn endpoints_clock_regression_and_long_gaps() {
        let mut value = MotionValue::new(12.0).unwrap();
        assert_eq!(
            value.animate_to(92.0, LINEAR, 100, false).unwrap(),
            Frame {
                value: 12.0,
                active: true
            }
        );
        assert_eq!(
            value.tick(150, false),
            Frame {
                value: 32.0,
                active: true
            }
        );
        assert_eq!(value.tick(120, false).value, 32.0);
        assert_eq!(
            value.tick(u64::MAX, false),
            Frame {
                value: 92.0,
                active: false
            }
        );
    }

    #[test]
    fn retargets_without_jumps_and_repeated_renders_do_not_restart() {
        let mut value = MotionValue::new(12.0).unwrap();
        value.animate_to(92.0, LINEAR, 0, false).unwrap();
        assert_eq!(
            value.animate_to(92.0, LINEAR, 50, false).unwrap().value,
            32.0
        );
        assert_eq!(
            value.animate_to(-8.0, LINEAR, 100, false).unwrap().value,
            52.0
        );
        assert_eq!(value.tick(150, false).value, 37.0);
        assert_eq!(
            value.tick(300, false),
            Frame {
                value: -8.0,
                active: false
            }
        );
    }

    #[test]
    fn reduced_motion_zero_duration_and_cancellation_stop_frames() {
        let mut value = MotionValue::new(12.0).unwrap();
        assert!(!value.animate_to(92.0, LINEAR, 0, true).unwrap().active);
        assert_eq!(value.tick(10, false).value, 92.0);
        value.animate_to(12.0, LINEAR, 10, false).unwrap();
        assert_eq!(
            value.tick(60, true),
            Frame {
                value: 12.0,
                active: false
            }
        );
        assert_eq!(value.tick(70, false).value, 12.0);
        assert_eq!(
            value
                .animate_to(
                    50.0,
                    Transition {
                        duration_ms: 0,
                        ..LINEAR
                    },
                    70,
                    false
                )
                .unwrap(),
            Frame {
                value: 50.0,
                active: false
            }
        );
        value.animate_to(90.0, LINEAR, 70, false).unwrap();
        value.tick(120, false);
        assert_eq!(
            value.cancel(),
            Frame {
                value: 60.0,
                active: false
            }
        );
        assert_eq!(value.tick(400, false).value, 60.0);
    }

    #[test]
    fn easing_is_not_accidentally_linear() {
        assert_eq!(Easing::Linear.sample(0.25), 0.25);
        assert_eq!(Easing::EaseInOut.sample(0.25), 0.125);
        assert_eq!(Easing::Quadratic.sample(0.25), 0.0625);
        assert_eq!(Easing::EaseOut.sample(0.25), 0.7626953125);
        for easing in [
            Easing::Linear,
            Easing::EaseInOut,
            Easing::Quadratic,
            Easing::EaseOut,
        ] {
            assert_eq!(easing.sample(-1.0), 0.0);
            assert_eq!(easing.sample(2.0), 1.0);
        }
        assert!("typo".parse::<Easing>().is_err());
    }

    #[test]
    fn non_finite_inputs_do_not_poison_running_motion() {
        assert!(MotionValue::new(f64::NAN).is_err());
        let mut value = MotionValue::new(12.0).unwrap();
        value.animate_to(92.0, LINEAR, 0, false).unwrap();
        assert!(value.animate_to(f64::INFINITY, LINEAR, 50, false).is_err());
        assert_eq!(value.tick(100, false).value, 52.0);
    }
}
