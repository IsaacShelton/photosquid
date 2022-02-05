use enum_as_inner::EnumAsInner;
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use nalgebra_glm as glm;

#[derive(Copy, Clone, EnumAsInner)]
pub enum Interaction {
    PreClick,
    Click(ClickInteraction),
    MouseRelease(MouseReleaseInteraction),
    Drag(DragInteraction),
    Key(KeyInteraction),
}

#[derive(Copy, Clone)]
pub struct ClickInteraction {
    pub button: MouseButton,
    pub position: glm::Vec2,
}

#[derive(Copy, Clone)]
pub struct MouseReleaseInteraction {
    pub button: MouseButton,
    pub position: glm::Vec2,
}

#[derive(Copy, Clone)]
pub struct DragInteraction {
    pub delta: glm::Vec2,
    pub start: glm::Vec2,
    pub current: glm::Vec2,
}

#[derive(Copy, Clone)]
pub struct KeyInteraction {
    pub virtual_keycode: VirtualKeyCode,
}
