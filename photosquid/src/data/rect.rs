use crate::{
    color::Color,
    smooth::{Lerpable, MultiLerp, NoLerp},
};
use angular_units::Rad;
use lyon::path::builder::BorderRadii as LyonBorderRadii;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct RectData {
    pub position: MultiLerp<glm::Vec2>,
    pub size: glm::Vec2,
    pub color: NoLerp<Color>,
    pub rotation: Rad<f32>,
    pub radii: BorderRadii,
    pub is_viewport: bool,
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Default, Serialize, Deserialize)]
pub struct BorderRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl BorderRadii {
    pub fn new(radius: f32) -> Self {
        let r = radius.abs();
        BorderRadii {
            top_left: r,
            top_right: r,
            bottom_left: r,
            bottom_right: r,
        }
    }
}

impl From<BorderRadii> for LyonBorderRadii {
    fn from(radii: BorderRadii) -> Self {
        LyonBorderRadii {
            top_left: radii.top_left,
            top_right: radii.top_right,
            bottom_left: radii.bottom_left,
            bottom_right: radii.bottom_right,
        }
    }
}

impl std::fmt::Display for BorderRadii {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BorderRadii({}, {}, {}, {})",
            self.top_left, self.top_right, self.bottom_left, self.bottom_right
        )
    }
}

impl Lerpable for RectData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        Self {
            position: self.position.lerp(&other.position, scalar),
            size: self.size.lerp(&other.size, scalar),
            rotation: self.rotation.lerp(&other.rotation, scalar),
            color: self.color.lerp(&other.color, scalar),
            radii: self.radii.lerp(&other.radii, scalar),
            is_viewport: self.is_viewport,
        }
    }
}

impl Lerpable for BorderRadii {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        BorderRadii {
            top_left: self.top_left.lerp(&other.top_left, scalar),
            top_right: self.top_right.lerp(&other.top_right, scalar),
            bottom_left: self.bottom_left.lerp(&other.bottom_left, scalar),
            bottom_right: self.bottom_right.lerp(&other.bottom_right, scalar),
        }
    }
}
