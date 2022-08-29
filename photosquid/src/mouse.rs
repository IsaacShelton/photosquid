use glium::glutin::dpi::LogicalPosition;
use nalgebra_glm as glm;

pub trait OnScreen {
    fn on_screen(self) -> glm::Vec2;
}

impl OnScreen for LogicalPosition<f32> {
    fn on_screen(self) -> glm::Vec2 {
        glm::vec2(self.x, self.y)
    }
}
