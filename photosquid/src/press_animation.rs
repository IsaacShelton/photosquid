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
}

impl PressAnimation {
    pub fn at_time(&self, focus: bool, t: f32) -> AnimationMoment {
        match self {
            Self::None => Default::default(),
            Self::Deform => {
                if focus {
                    AnimationMoment {
                        relative_scale: if t < 0.5 { (2.0 * t).exponential_out() } else { 1.0 },
                        rotation: 0.25
                            * if t < 0.5 {
                                (std::f32::consts::FRAC_PI_2 * -2.0 * (1.0 - t * 2.0)).sin()
                            } else {
                                0.0
                            },
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::Cycle => {
                if focus {
                    AnimationMoment {
                        rotation: if t < 0.5 { (2.0 * t).exponential_out() * std::f32::consts::TAU } else { 0.0 },
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::HalfCycle => {
                if focus {
                    AnimationMoment {
                        rotation: if t < 0.5 {
                            (2.0 * t).exponential_out() * std::f32::consts::PI + std::f32::consts::PI
                        } else {
                            0.0
                        },
                        ..Default::default()
                    }
                } else {
                    Default::default()
                }
            }
            Self::Fall => AnimationMoment {
                backwards_rotation: if focus { t.bounce_out() } else { 1.0 - t.exponential_out() },
                ..Default::default()
            },
        }
    }
}

pub struct AnimationMoment {
    pub backwards_rotation: f32,
    pub rotation: f32,
    pub relative_scale: f32,
}

impl Default for AnimationMoment {
    fn default() -> Self {
        Self {
            backwards_rotation: 0.0,
            rotation: 0.0,
            relative_scale: 1.0,
        }
    }
}
