use interpolation::Ease;

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

pub trait PressAnimation {
    fn at_time(&self, focus: bool, t: f32) -> AnimationMoment;
}

pub struct NoPressAnimation {}

impl PressAnimation for NoPressAnimation {
    fn at_time(&self, _focus: bool, _t: f32) -> AnimationMoment {
        Default::default()
    }
}

pub struct DeformPressAnimation {}

impl PressAnimation for DeformPressAnimation {
    fn at_time(&self, focus: bool, t: f32) -> AnimationMoment {
        if focus == false {
            return Default::default();
        }

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
    }
}

pub struct CyclePressAnimation {}

impl PressAnimation for CyclePressAnimation {
    fn at_time(&self, focus: bool, t: f32) -> AnimationMoment {
        if focus == false {
            return Default::default();
        }

        AnimationMoment {
            rotation: if t < 0.5 { (2.0 * t).exponential_out() * std::f32::consts::TAU } else { 0.0 },
            ..Default::default()
        }
    }
}

pub struct FallPressAnimation {}

impl PressAnimation for FallPressAnimation {
    fn at_time(&self, focus: bool, t: f32) -> AnimationMoment {
        AnimationMoment {
            backwards_rotation: if focus { t.bounce_out() } else { 1.0 - t.exponential_out() },
            ..Default::default()
        }
    }
}
