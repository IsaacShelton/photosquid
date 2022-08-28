use nalgebra_glm as glm;

#[derive(Copy, Clone, Default)]
pub struct DilateBehavior {
    pub origin: glm::Vec2,
    pub start: glm::Vec2,
    pub point: glm::Vec2,
}

pub struct Expression {
    pub position: glm::Vec2,
    pub total_scale_factor: f32,
}

impl DilateBehavior {
    // Returns new absolute position
    pub fn express(&self, current: &glm::Vec2) -> Expression {
        use crate::math::DivOrZero;
        let origin = &self.origin;
        let start = &self.start;
        let angle = (start.y - origin.y).atan2(start.x - origin.x);
        let factor = glm::distance(current, origin).div_or_zero(glm::distance(&self.point, origin));
        let new_distance = factor * glm::distance(start, origin);

        Expression {
            position: origin + new_distance * glm::vec2(angle.cos(), angle.sin()),
            total_scale_factor: factor,
        }
    }
}
