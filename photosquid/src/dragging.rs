use crate::interaction::{DragInteraction, Interaction};
use glium::glutin::dpi::LogicalPosition;
use nalgebra_glm as glm;

pub struct Dragging {
    pub down: glm::Vec2,
    pub current: glm::Vec2,
    pub last: glm::Vec2,
}

impl Dragging {
    pub fn new(mouse_position: LogicalPosition<f32>) -> Self {
        let position: glm::Vec2 = glm::vec2(mouse_position.x, mouse_position.y);

        Self {
            down: position,
            current: position,
            last: position,
        }
    }

    pub fn update(&mut self, mouse_position: glm::Vec2) {
        self.last = self.current;
        self.current = mouse_position;
    }

    pub fn get_delta(&self) -> glm::Vec2 {
        self.current - self.last
    }

    pub fn to_interaction(&self) -> Interaction {
        Interaction::Drag(DragInteraction {
            delta: self.get_delta(),
            start: self.down,
            current: self.current,
        })
    }
}
