use crate::{
    color::Color,
    smooth::{Lerpable, MultiLerp, NoLerp},
};
use angular_units::Rad;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct CircleData {
    pub position: MultiLerp<glm::Vec2>,
    pub radius: f32,
    pub color: NoLerp<Color>,
    pub virtual_rotation: Rad<f32>,
}

impl Lerpable for CircleData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        Self {
            position: self.position.lerp(&other.position, scalar),
            radius: self.radius.lerp(&other.radius, scalar),
            color: self.color.lerp(&other.color, scalar),
            virtual_rotation: self.virtual_rotation.lerp(&other.virtual_rotation, scalar),
        }
    }
}
