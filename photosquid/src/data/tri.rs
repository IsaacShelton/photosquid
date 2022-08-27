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

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        Self {
            p1: self.p1.lerp(&other.p1, scalar),
            p2: self.p2.lerp(&other.p2, scalar),
            p3: self.p3.lerp(&other.p3, scalar),
            position: self.position.lerp(&other.position, scalar),
            rotation: self.rotation.lerp(&other.rotation, scalar),
            color: self.color.lerp(&other.color, scalar),
        }
    }
}
