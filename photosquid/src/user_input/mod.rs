mod button;
mod checkbox;
mod text_input;

pub use button::Button;
pub use checkbox::Checkbox;
pub use text_input::TextInput;

use crate::{
    aabb::AABB,
    capture::{Capture, KeyCapture},
    render_ctx::RenderCtx,
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

    Button(Button),
}

impl UserInput {
    pub fn click(&mut self, mouse_button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        match self {
            Self::TextInput(text_input) => text_input.click(mouse_button, position, area),
            Self::Checkbox(checkbox) => checkbox.click(mouse_button, position, area),
            Self::Button(button) => button.click(mouse_button, position, area),
        }
    }

    pub fn key_press(&mut self, virtual_keycode: VirtualKeyCode, shift: bool) -> KeyCapture {
        match self {
            Self::TextInput(text_input) => text_input.key_press(virtual_keycode, shift),
            Self::Checkbox(..) => KeyCapture::Miss,
            Self::Button(..) => KeyCapture::Miss,
        }
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        match self {
            Self::TextInput(text_input) => text_input.render(ctx, text_system, font, area),
            Self::Checkbox(checkbox) => checkbox.render(ctx, text_system, font, area),
            Self::Button(button) => button.render(ctx, text_system, font, area),
        }
    }

    pub fn unfocus(&mut self) {
        match self {
            Self::TextInput(text_input) => text_input.unfocus(),
            Self::Checkbox(..) => (),
            Self::Button(..) => (),
        }
    }
}
