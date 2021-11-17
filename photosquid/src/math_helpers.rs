use nalgebra_glm as glm;

pub trait DivOrZero {
    fn div_or_zero(&self, denominator: Self) -> Self;
}

impl DivOrZero for f32 {
    fn div_or_zero(&self, denominator: f32) -> f32 {
        if denominator == 0.0 {
            0.0
        } else {
            *self / denominator
        }
    }
}

impl DivOrZero for f64 {
    fn div_or_zero(&self, denominator: f64) -> f64 {
        if denominator == 0.0 {
            0.0
        } else {
            *self / denominator
        }
    }
}

pub trait AsAngle {
    fn as_angle(&self) -> f32;
}

impl AsAngle for glm::Vec2 {
    fn as_angle(&self) -> f32 {
        self.y.atan2(self.x)
    }
}

pub fn angle_difference(alpha: f32, beta: f32) -> f32 {
    use std::f32::consts::{PI, TAU};

    let difference = (beta - alpha + PI) % TAU - PI;

    if difference < -PI {
        difference + TAU
    } else {
        difference
    }
}
