use crate::{
    color::Color,
    smooth::{Lerpable, MultiLerp, NoLerp},
};
use angular_units::Rad;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct TriData {
    pub p1: MultiLerp<glm::Vec2>,
    pub p2: MultiLerp<glm::Vec2>,
    pub p3: MultiLerp<glm::Vec2>,
    pub position: MultiLerp<glm::Vec2>,
    pub color: NoLerp<Color>,
    pub rotation: Rad<f32>,
}

impl Lerpable for TriData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            p1: Lerpable::lerp(&self.p1, &other.p1, scalar),
            p2: Lerpable::lerp(&self.p2, &other.p2, scalar),
            p3: Lerpable::lerp(&self.p3, &other.p3, scalar),
            position: self.position.lerp(&other.position, scalar),
            rotation: angular_units::Interpolate::interpolate(&self.rotation, &other.rotation, *scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
        }
    }
}
