use angular_units::Rad;
use nalgebra_glm as glm;

pub enum Operation {
    Rotate { point: glm::Vec2, rotation: Rad<f32> },
    Scale { point: glm::Vec2, origin: glm::Vec2 },
    Spread { point: glm::Vec2, origin: glm::Vec2 },
    Revolve { point: glm::Vec2, origin: glm::Vec2 },
    Dilate { point: glm::Vec2, origin: glm::Vec2 },
}
