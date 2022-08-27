use crate::math_helpers::AsAngle;
use angular_units::{Angle, Interpolate};
use nalgebra_glm as glm;

pub trait CircleLerpable {
    type Origin: Copy + Clone;
    type Scalar: Copy + Clone;

    fn circle_lerp(&self, other: &Self, origin: Self::Origin, scalar: Self::Scalar) -> Self;
}

impl CircleLerpable for glm::Vec2 {
    type Origin = glm::Vec2;
    type Scalar = f32;

    fn circle_lerp(&self, other: &Self, origin: Self::Origin, scalar: Self::Scalar) -> Self {
        let distance = glm::distance(self, &origin);
        let alpha = (*self - origin).as_angle();
        let beta = (*other - origin).as_angle();
        let angle = alpha.interpolate(&beta, scalar);

        origin + distance * glm::vec2(angle.cos(), angle.sin())
    }
}
