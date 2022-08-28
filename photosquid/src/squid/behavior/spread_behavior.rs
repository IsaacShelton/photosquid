use nalgebra_glm as glm;

#[derive(Copy, Clone, Default)]
pub struct SpreadBehavior {
    pub origin: glm::Vec2,
    pub start: glm::Vec2,
    pub point: glm::Vec2,
}

impl SpreadBehavior {
    // Returns new absolute position
    pub fn express(&self, current: &glm::Vec2) -> glm::Vec2 {
        use crate::math::DivOrZero;
        use glm::distance;

        let origin = &self.origin;
        let start = &self.start;
        let angle = (start.y - origin.y).atan2(start.x - origin.x);
        let new_distance = distance(current, origin).div_or_zero(distance(&self.point, origin)) * distance(start, origin);
        origin + new_distance * glm::vec2(angle.cos(), angle.sin())
    }
}
