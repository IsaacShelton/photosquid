use crate::{accumulator::Accumulator, interaction_options::InteractionOptions};
use nalgebra_glm as glm;

pub struct TranslateBehavior {
    pub moving: bool,
    pub accumulator: Accumulator<glm::Vec2>,
}

impl TranslateBehavior {
    // Returns delta position
    pub fn express(&mut self, raw_delta: &glm::Vec2, options: &InteractionOptions) -> glm::Vec2 {
        self.accumulator.accumulate(raw_delta, options.translation_snapping).unwrap_or_default()
    }
}

impl Default for TranslateBehavior {
    fn default() -> Self {
        Self {
            moving: false,
            accumulator: Accumulator::new(),
        }
    }
}
