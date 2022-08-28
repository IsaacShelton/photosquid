use angular_units::{Angle, Rad};
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
    fn as_angle(&self) -> Rad<f32>;
}

impl AsAngle for glm::Vec2 {
    fn as_angle(&self) -> Rad<f32> {
        Rad(self.y.atan2(self.x))
    }
}

pub fn angle_difference(alpha: Rad<f32>, beta: Rad<f32>) -> Rad<f32> {
    use std::f32::consts::{PI, TAU};

    let alpha = alpha.scalar();
    let beta = beta.scalar();

    let difference = (beta - alpha + PI) % TAU - PI;

    Rad(if difference < -PI { difference + TAU } else { difference })
}

pub fn get_point_delta_rotation(screen_position: &glm::Vec2, mouse_position: &glm::Vec2, old_rotation: Rad<f32>) -> Rad<f32> {
    let new_rotation = Rad(-1.0 * (mouse_position.y - screen_position.y).atan2(mouse_position.x - screen_position.x));
    angle_difference(old_rotation, new_rotation)
}
