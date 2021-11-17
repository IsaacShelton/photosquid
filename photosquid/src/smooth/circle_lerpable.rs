use nalgebra_glm as glm;

pub trait CircleLerpable {
    type Origin;
    type Scalar;

    fn circle_lerp(&self, other: &Self, origin: &Self::Origin, scalar: &Self::Scalar) -> Self;
}

impl CircleLerpable for glm::Vec2 {
    type Origin = glm::Vec2;
    type Scalar = f32;

    fn circle_lerp(&self, other: &Self, origin: &Self::Origin, scalar: &Self::Scalar) -> Self {
        use crate::math_helpers::{angle_difference, AsAngle};

        // Assert that previous and next points lie roughly on a circular path centered at `origin`
        // assert!(almost::equal(glm::distance(origin, self), glm::distance(origin, other)));

        let distance = glm::distance(self, origin);
        let alpha = (*self - *origin).as_angle();
        let beta = (*other - *origin).as_angle();
        let angle = scalar * angle_difference(alpha, beta) + alpha;

        *origin + distance * glm::vec2(angle.cos(), angle.sin())
    }
}
