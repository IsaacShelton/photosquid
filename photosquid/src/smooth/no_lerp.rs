use serde::{Deserialize, Serialize};

use super::Lerpable;

#[derive(Default, Copy, Clone, Serialize, Deserialize)]
pub struct NoLerp<T: Lerpable + Copy>(pub T);

impl<T: Lerpable + Copy> Lerpable for NoLerp<T> {
    type Scalar = T::Scalar;

    fn lerp(&self, other: &Self, _scalar: &Self::Scalar) -> Self {
        // Don't do lerping shape color changes
        *other
    }
}
