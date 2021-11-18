use angular_units::{Angle, Rad};
use nalgebra_glm as glm;

#[derive(Copy, Clone, Default)]
pub struct Accumulator<T: Accumulatable> {
    value: T,
}

impl<T: Accumulatable> Accumulator<T> {
    pub fn new() -> Self {
        Self { value: T::zero() }
    }

    pub fn clear(&mut self) {
        self.value = T::zero();
    }

    pub fn accumulate(&mut self, other: &T, threshold: T::Threshold) -> Option<T> {
        self.value.accumulate(other, threshold)
    }

    pub fn residue(&self) -> &T {
        &self.value
    }
}

pub trait Accumulatable: Sized {
    type Threshold;

    fn zero() -> Self;

    fn accumulate(&mut self, other: &Self, threshold: Self::Threshold) -> Option<Self>;
}

impl Accumulatable for f32 {
    type Threshold = f32;

    fn zero() -> Self {
        0.0
    }

    fn accumulate(&mut self, other: &Self, threshold: Self::Threshold) -> Option<Self> {
        // threshold of PI
        // PI => PI
        // -PI => -PI
        // PI/2 => PI
        // -PI/2 => -PI
        // -PI/3 => 0
        // PI/3 => 0

        // f(-x) = -f(x)

        *self += *other;

        let result = if threshold > 0.0 {
            (((*self + 0.5 * threshold) / threshold).floor()) * threshold
        } else {
            *other
        };

        if result == 0.0 {
            None
        } else {
            *self -= result;
            Some(result)
        }
    }
}

impl Accumulatable for glm::Vec2 {
    type Threshold = f32;

    fn zero() -> Self {
        glm::zero()
    }

    fn accumulate(&mut self, delta: &Self, threshold: Self::Threshold) -> Option<Self> {
        *self += delta;

        let result = if threshold > 0.0 {
            glm::vec2(
                (((self.x + 0.5 * threshold) / threshold).floor()) * threshold,
                (((self.y + 0.5 * threshold) / threshold).floor()) * threshold,
            )
        } else {
            *delta
        };

        if result == glm::zero::<glm::Vec2>() {
            None
        } else {
            *self -= result;
            Some(result)
        }
    }
}

impl Accumulatable for Rad<f32> {
    type Threshold = Rad<f32>;

    fn zero() -> Self {
        Rad(0.0)
    }

    fn accumulate(&mut self, other: &Self, threshold: Self::Threshold) -> Option<Self> {
        // threshold of PI
        // PI => PI
        // -PI => -PI
        // PI/2 => PI
        // -PI/2 => -PI
        // -PI/3 => 0
        // PI/3 => 0

        // f(-x) = -f(x)

        *self += *other;

        let result = if threshold.scalar() > 0.0 {
            (((self.scalar() + 0.5 * threshold.scalar()) / threshold.scalar()).floor()) * threshold.scalar()
        } else {
            other.scalar()
        };

        if result == 0.0 {
            None
        } else {
            *self -= Rad(result);
            Some(Rad(result))
        }
    }
}
