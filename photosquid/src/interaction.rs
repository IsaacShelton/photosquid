use glium::glutin::event::{MouseButton, VirtualKeyCode};
use nalgebra_glm as glm;

pub enum Interaction {
    PreClick,
    Click { button: MouseButton, position: glm::Vec2 },
    MouseRelease { button: MouseButton, position: glm::Vec2 },
    Drag { delta: glm::Vec2, start: glm::Vec2, current: glm::Vec2 },
    Key { virtual_keycode: VirtualKeyCode },
}
