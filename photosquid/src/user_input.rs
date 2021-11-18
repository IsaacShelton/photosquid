use crate::{
    aabb::AABB,
    capture::{Capture, KeyCapture},
    checkbox::Checkbox,
    render_ctx::RenderCtx,
    text_input::TextInput,
};
use enum_as_inner::EnumAsInner;
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

#[derive(EnumAsInner)]
#[allow(clippy::large_enum_variant)]
pub enum UserInput {
    TextInput(TextInput),

    #[allow(dead_code)]
    Checkbox(Checkbox),
}

impl UserInput {
    pub fn click(&mut self, button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        match self {
            Self::TextInput(text_input) => text_input.click(button, position, area),
            Self::Checkbox(checkbox) => checkbox.click(button, position, area),
        }
    }

    pub fn key_press(&mut self, virtual_keycode: VirtualKeyCode, shift: bool) -> KeyCapture {
        match self {
            Self::TextInput(text_input) => text_input.key_press(virtual_keycode, shift),
            Self::Checkbox(..) => KeyCapture::Miss,
        }
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        match self {
            Self::TextInput(text_input) => text_input.render(ctx, text_system, font, area),
            Self::Checkbox(checkbox) => checkbox.render(ctx, text_system, font, area),
        }
    }

    pub fn unfocus(&mut self) {
        match self {
            Self::TextInput(text_input) => text_input.unfocus(),
            Self::Checkbox(..) => (),
        }
    }
}
