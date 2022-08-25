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

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            position: Lerpable::lerp(&self.position, &other.position, scalar),
            radius: interpolation::Lerp::lerp(&self.radius, &other.radius, scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
            virtual_rotation: angular_units::Interpolate::interpolate(&self.virtual_rotation, &other.virtual_rotation, *scalar),
        }
    }
}
