use crate::{
    color::Color,
    smooth::{Lerpable, MultiLerp, NoLerp},
};
use angular_units::Rad;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct RectData {
    pub position: MultiLerp<glm::Vec2>,
    pub size: glm::Vec2,
    pub color: NoLerp<Color>,
    pub rotation: Rad<f32>,
    pub radii: f32,
}

impl Lerpable for RectData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            position: self.position.lerp(&other.position, scalar),
            size: Lerpable::lerp(&self.size, &other.size, scalar),
            rotation: angular_units::Interpolate::interpolate(&self.rotation, &other.rotation, *scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
            radii: interpolation::Lerp::lerp(&self.radii, &other.radii, scalar),
        }
    }
}
