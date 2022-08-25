use super::{CircleLerpable, Lerpable};
use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, EnumAsInner, Serialize, Deserialize)]
pub enum MultiLerp<T: Lerpable + CircleLerpable + Copy + Clone> {
    From(T),
    Linear(T),
    Circle(T, T::Origin),
}

impl<T: Lerpable + CircleLerpable + Copy + Clone> MultiLerp<T> {
    pub fn reveal(&self) -> T {
        match self {
            Self::From(value) => *value,
            Self::Linear(value) => *value,
            Self::Circle(value, ..) => *value,
        }
    }
}

impl<T: Lerpable<Scalar = f32> + CircleLerpable<Scalar = f32> + Copy + Clone> Lerpable for MultiLerp<T> {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self::From(match other {
            Self::From(value) => *value,
            Self::Linear(value) => self.reveal().lerp(value, scalar),
            Self::Circle(value, origin) => CircleLerpable::circle_lerp(&self.reveal(), value, origin, scalar),
        })
    }
}

impl<T: Lerpable + CircleLerpable + Copy + Default> Default for MultiLerp<T> {
    fn default() -> Self {
        Self::Linear(T::default())
    }
}
