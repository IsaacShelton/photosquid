use crate::{accumulator::Accumulator, interaction_options::InteractionOptions, math_helpers::angle_difference};
use angular_units::{Angle, Rad};
use nalgebra_glm as glm;

#[derive(Copy, Clone)]
pub struct RevolveBehavior {
    revolving: bool,
    origin: glm::Vec2,
    start: glm::Vec2,
    point: glm::Vec2,
    accumulator: Accumulator<Rad<f32>>,
    rotation: Rad<f32>,
}

pub struct Expression {
    pub origin_rotation: Rad<f32>,
    pub origin: glm::Vec2,
    pub start: glm::Vec2,
    pub delta_object_rotation: Rad<f32>,
}

impl Expression {
    pub fn apply_origin_rotation_to_center(&self) -> glm::Vec2 {
        use crate::math_helpers::AsAngle;
        let distance = glm::distance(&self.origin, &self.start);
        let object_angle = (self.start - self.origin).as_angle() - self.origin_rotation;
        self.origin + distance * glm::vec2(object_angle.cos(), object_angle.sin())
    }
}

impl RevolveBehavior {
    // Returns origin point to rotate around a certain amount
    pub fn express(&mut self, current: &glm::Vec2, options: &InteractionOptions) -> Option<Expression> {
        use crate::math_helpers::AsAngle;

        if !self.revolving {
            None
        } else {
            let mu0 = (self.point - self.origin).as_angle();
            let mu1 = (current - self.origin).as_angle();
            let total_delta_mu = mu0 - mu1;

            let raw_delta_rotation = angle_difference(self.rotation + *self.accumulator.residue(), total_delta_mu);
            let delta_rotation = self.accumulator.accumulate(&raw_delta_rotation, options.rotation_snapping).unwrap_or_default();

            self.rotation += delta_rotation;

            Some(Expression {
                origin_rotation: self.rotation,
                origin: self.origin,
                start: self.start,
                delta_object_rotation: delta_rotation,
            })
        }
    }

    pub fn set(&mut self, origin: &glm::Vec2, start: &glm::Vec2, point: &glm::Vec2) {
        self.accumulator.clear();
        self.origin = *origin;
        self.start = *start;
        self.point = *point;
        self.revolving = true;
        self.rotation = Rad(0.0);
    }

    pub fn unset(&mut self) {
        self.revolving = false;
    }
}

impl Default for RevolveBehavior {
    fn default() -> Self {
        Self {
            revolving: false,
            origin: glm::zero(),
            start: glm::zero(),
            point: glm::zero(),
            accumulator: Accumulator::new(),
            rotation: Rad(0.0),
        }
    }
}
