use super::Lerpable;
use interpolation::Ease;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize)]
pub struct Smooth<T: Lerpable + Copy> {
    data: T,

    #[serde(skip)]
    previous: T,

    #[serde(skip, default = "Instant::now")]
    changed: Instant,

    #[serde(skip, default = "default_smooth_duration")]
    duration: Duration,
}

pub fn default_smooth_duration() -> Duration {
    Duration::from_millis(500)
}

impl<T: Lerpable + Copy> Smooth<T>
where
    <T as Lerpable>::Scalar: From<f32>,
{
    pub fn new(initial: T, duration: Option<Duration>) -> Self {
        Self {
            data: initial,
            previous: initial,
            changed: Instant::now(),
            duration: duration.unwrap_or_else(default_smooth_duration),
        }
    }

    pub fn get_real(&self) -> &T {
        &self.data
    }

    pub fn get_animated(&self) -> T {
        self.previous.lerp(&self.data, self.t())
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
