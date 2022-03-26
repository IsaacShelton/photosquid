use angular_units::Rad;
use interpolation::Ease;

#[derive(Copy, Clone)]
pub enum PressAnimation {
    #[allow(dead_code)]
    None,

    Deform,

    #[allow(dead_code)]
    Cycle,

    HalfCycle,

    #[allow(dead_code)]
    Fall,

    Scale,
}

impl PressAnimation {
    pub fn at_time(&self, focus: bool, t: f32) -> AnimationMoment {
        use std::f32::consts::{FRAC_PI_2, PI};

        match self {
            Self::None => Default::default(),
            Self::Deform => {
                if focus {
                    AnimationMoment {
                        relative_scale: if t < 0.5 { (2.0 * t).exponential_out() } else { 1.0 },
                        rotation: Rad(0.25 * if t < 0.5 { (FRAC_PI_2 * -2.0 * (1.0 - t * 2.0)).sin() } else { 0.0 }),
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::Cycle => {
                if focus {
                    AnimationMoment {
                        rotation: Rad(if t < 0.5 { (2.0 * t).exponential_out() * std::f32::consts::TAU } else { 0.0 }),
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::HalfCycle => {
                if focus {
                    AnimationMoment {
                        rotation: Rad(if t < 0.5 { (2.0 * t).exponential_out() * PI + PI } else { 0.0 }),
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::Fall => AnimationMoment {
                backwards_rotation: Rad(if focus { t.bounce_out() } else { 1.0 - t.exponential_out() }),
                ..Default::default()
            },
            Self::Scale => {
                if focus {
                    AnimationMoment {
                        relative_scale: if t < 0.5 { (2.0 * t).exponential_out() } else { 1.0 },
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
        }
    }
}

pub struct AnimationMoment {
    pub backwards_rotation: Rad<f32>,
    pub rotation: Rad<f32>,
    pub relative_scale: f32,
}

impl Default for AnimationMoment {
    fn default() -> Self {
        Self {
            backwards_rotation: Rad(0.0),
            rotation: Rad(0.0),
            relative_scale: 1.0,
        }
    }
}
