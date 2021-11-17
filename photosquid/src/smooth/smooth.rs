use super::Lerpable;
use interpolation::Ease;
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

    pub fn manual_get_real(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn manual_get_previous(&mut self) -> &mut T {
        &mut self.previous
    }
}
