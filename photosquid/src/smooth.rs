use interpolation::Ease;
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Smooth<T: Lerpable + Copy> {
    data: T,
    previous: T,
    changed: Instant,
    duration: Duration,
}

impl<T: Lerpable + Copy> Smooth<T>
where
    <T as Lerpable>::Scalar: From<f32>,
{
    pub fn new(initial: T, duration: Duration) -> Self {
        Self {
            data: initial,
            previous: initial,
            changed: Instant::now(),
            duration,
        }
    }

    pub fn get_real(&self) -> &T {
        &self.data
    }

    pub fn get_animated(&self) -> T {
        Lerpable::lerp(&self.previous, &self.data, &self.t())
    }

    pub fn t(&self) -> <T as Lerpable>::Scalar {
        (self.changed.elapsed().as_millis() as f32 / self.duration.as_millis() as f32)
            .clamp(0.0, 1.0)
            .exponential_out()
            .into()
    }

    pub fn set(&mut self, new: T) {
        self.previous = self.get_animated();
        self.data = new;
        self.changed = Instant::now();
    }
}

// Since we cannot extend interpolation::Lerp, we must make our own trait
pub trait Lerpable {
    type Scalar;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self;
}

impl Lerpable for f32 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, scalar)
    }
}

impl Lerpable for f64 {
    type Scalar = f64;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, scalar)
    }
}

impl Lerpable for glm::Vec2 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        glm::vec2(
            interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            interpolation::Lerp::lerp(&self.y, &other.y, scalar),
        )
    }
}

impl Lerpable for glm::Vec3 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        glm::vec3(
            interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            interpolation::Lerp::lerp(&self.y, &other.y, scalar),
            interpolation::Lerp::lerp(&self.z, &other.z, scalar),
        )
    }
}

impl Lerpable for crate::color::Color {
    type Scalar = f32;

    fn lerp(&self, other: &Self, _scalar: &Self::Scalar) -> Self {
        // Don't do lerping shape color changes
        *other
    }
}
