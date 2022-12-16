use std::convert::TryInto;

use crate::{
    color::Color,
    smooth::{Lerpable, MultiLerp, NoLerp},
};
use angular_units::Rad;
use itertools::Itertools;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct TriData {
    pub p: [MultiLerp<glm::Vec2>; 3],
    pub position: MultiLerp<glm::Vec2>,
    pub color: NoLerp<Color>,
    pub rotation: Rad<f32>,
}

impl Lerpable for TriData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        let p: [MultiLerp<glm::Vec2>; 3] = self
            .p
            .iter()
            .zip(other.p)
            .map(|(self_p, other_p)| self_p.lerp(&other_p, scalar))
            .collect_vec()
            .try_into()
            .unwrap_or_default();

        Self {
            p,
            position: self.position.lerp(&other.position, scalar),
            rotation: self.rotation.lerp(&other.rotation, scalar),
            color: self.color.lerp(&other.color, scalar),
        }
    }
}
