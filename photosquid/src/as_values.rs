use nalgebra_glm as glm;

use crate::color::Color;

pub trait AsValues {
    type ValuesType;

    fn as_values(&self) -> Self::ValuesType;
}

impl AsValues for glm::Mat4 {
    type ValuesType = [[f32; 4]; 4];

    fn as_values(&self) -> Self::ValuesType {
        return *self.as_ref();
    }
}

impl AsValues for Color {
    type ValuesType = [f32; 4];

    fn as_values(&self) -> Self::ValuesType {
        self.into()
    }
}
